use crate::psbt::fee::DUST_THRESHOLD;
use crate::psbt::types::{BuilderError, ScriptType, Utxo};
use crate::psbt::vbytes;

/// Maximum candidates for Branch-and-Bound to avoid exponential blowup
const MAX_BNB_UTXOS: usize = 100;
/// DFS iteration cap shared by Branch-and-Bound and CoinGrinder
const MAX_SEARCH_TRIES: u32 = 100_000;

/// Pre-computed output weights for vbytes estimation during coin selection.
/// Uses exact script sizes when available, falling back to template estimates.
#[derive(Debug, Clone)]
struct OutputWeights {
    /// Total weight of payment outputs (excluding change)
    payment_outputs_weight: u64,
    /// Weight of the change output
    change_output_weight: u64,
    /// Number of payment outputs
    num_payment_outputs: usize,
}

impl OutputWeights {
    /// Build from exact script hexes (preferred) or fall back to template types
    fn new(
        output_types: &[ScriptType],
        change_type: &ScriptType,
        output_script_hexes: Option<&[String]>,
        change_script_hex: Option<&str>,
    ) -> Self {
        let payment_outputs_weight = if let Some(hexes) = output_script_hexes {
            hexes
                .iter()
                .map(|h| vbytes::output_weight_for_script(h))
                .sum()
        } else {
            output_types.iter().map(|t| vbytes::output_weight(t)).sum()
        };

        let change_output_weight = if let Some(hex) = change_script_hex {
            vbytes::output_weight_for_script(hex)
        } else {
            vbytes::output_weight(change_type)
        };

        OutputWeights {
            payment_outputs_weight,
            change_output_weight,
            num_payment_outputs: output_types.len(),
        }
    }

    /// Compute vbytes for a given set of inputs WITH change
    fn vbytes_with_change(&self, input_types: &[ScriptType]) -> u64 {
        let has_segwit = input_types.iter().any(|t| t.is_segwit());
        let outputs_weight = self.payment_outputs_weight + self.change_output_weight;
        let overhead =
            vbytes::overhead_weight(input_types.len(), self.num_payment_outputs + 1, has_segwit);
        let inputs_weight: u64 = input_types.iter().map(|t| vbytes::input_weight(t)).sum();
        vbytes::weight_to_vbytes(overhead + inputs_weight + outputs_weight)
    }

    /// Compute vbytes for a given set of inputs WITHOUT change
    fn vbytes_no_change(&self, input_types: &[ScriptType]) -> u64 {
        let has_segwit = input_types.iter().any(|t| t.is_segwit());
        let overhead =
            vbytes::overhead_weight(input_types.len(), self.num_payment_outputs, has_segwit);
        let inputs_weight: u64 = input_types.iter().map(|t| vbytes::input_weight(t)).sum();
        vbytes::weight_to_vbytes(overhead + inputs_weight + self.payment_outputs_weight)
    }
}

/// The strategy that won coin selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    /// Single UTXO selection (exact or near-exact match)
    SingleUtxo,
    /// Branch-and-bound: changeless exact match
    BranchAndBound,
    /// CoinGrinder: minimum-weight input set with change
    CoinGrinder,
    /// Greedy: largest-first until funded
    Greedy,
}

impl Strategy {
    pub fn name(&self) -> &'static str {
        match self {
            Strategy::SingleUtxo => "single_utxo",
            Strategy::BranchAndBound => "branch_and_bound",
            Strategy::CoinGrinder => "coin_grinder",
            Strategy::Greedy => "greedy",
        }
    }
}

/// Result of coin selection
#[derive(Debug, Clone)]
pub struct CoinSelection {
    pub selected: Vec<Utxo>,
    pub total_value: u64,
    pub strategy: Strategy,
}

/// Candidate solution from coin selection
#[derive(Debug, Clone)]
struct Candidate {
    indices: Vec<usize>,
    total_value: u64,
    score: CandidateScore,
    /// Which strategy produced this candidate
    strategy: Strategy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CandidateScore {
    fee_sats: u64,
    input_count: usize,
    change_sats: u64,
}

impl CandidateScore {
    fn better_than(&self, other: &CandidateScore) -> bool {
        if self.fee_sats != other.fee_sats {
            return self.fee_sats < other.fee_sats;
        }
        if self.input_count != other.input_count {
            return self.input_count < other.input_count;
        }
        if self.change_sats != other.change_sats {
            return self.change_sats < other.change_sats;
        }
        false
    }
}

fn needed_amount(total_payments: u64, fee: u64) -> Option<u64> {
    total_payments.checked_add(fee)
}

/// Select coins using a multi-strategy approach optimized for a multi-objective score.
///
/// Strategy order (all tried, best score wins):
/// 1. Single UTXO (changeless or with change)
/// 2. Branch-and-bound for exact match (no change, minimal leftover burned as fee)
/// 3. CoinGrinder: DFS for minimum-weight input set with change
/// 4. Fallback: largest-first greedy
///
/// When `output_script_hexes` and `change_script_hex` are provided, exact script
/// sizes are used for vbytes estimation (tighter than template-based estimates).
pub fn select_coins(
    utxos: &[Utxo],
    total_payments: u64,
    fee_rate: f64,
    output_types: &[ScriptType],
    change_type: &ScriptType,
    max_inputs: Option<u32>,
) -> Result<CoinSelection, BuilderError> {
    select_coins_with_scripts(
        utxos,
        total_payments,
        fee_rate,
        output_types,
        change_type,
        max_inputs,
        None,
        None,
    )
}

/// Select coins with optional exact output script hex values for tighter fee estimation.
pub fn select_coins_with_scripts(
    utxos: &[Utxo],
    total_payments: u64,
    fee_rate: f64,
    output_types: &[ScriptType],
    change_type: &ScriptType,
    max_inputs: Option<u32>,
    output_script_hexes: Option<&[String]>,
    change_script_hex: Option<&str>,
) -> Result<CoinSelection, BuilderError> {
    if utxos.is_empty() {
        return Err(BuilderError::new(
            "INSUFFICIENT_FUNDS",
            "No UTXOs available",
        ));
    }

    let max = max_inputs.map(|m| m as usize).unwrap_or(utxos.len());

    // Pre-compute output weights for exact vbytes estimation
    let ow = OutputWeights::new(
        output_types,
        change_type,
        output_script_hexes,
        change_script_hex,
    );

    // Pre-compute per-UTXO effective values and filter out uneconomical UTXOs
    // (those where the cost to spend exceeds their value)
    let mut candidates: Vec<(usize, &Utxo, u64, i64)> = Vec::new(); // (index, utxo, input_weight, effective_value)
    for (i, utxo) in utxos.iter().enumerate() {
        let iw = vbytes::input_weight(&utxo.script_type);
        let spending_cost = ((iw as f64 * fee_rate) / 4.0).ceil() as i64;
        let effective_value = utxo.value_sats as i64 - spending_cost;
        candidates.push((i, utxo, iw, effective_value));
    }

    // Sort by effective value descending for greedy approaches
    candidates.sort_by(|a, b| b.3.cmp(&a.3));

    // Early rejection: if the sum of ALL UTXOs (raw value, ignoring fees)
    // can't even cover total_payments, no subset can possibly work either.

    let total_available = candidates
        .iter()
        .try_fold(0u64, |acc, (_, u, _, _)| acc.checked_add(u.value_sats))
        .ok_or_else(|| {
            BuilderError::new(
                "INVALID_FIXTURE",
                "Total UTXO amounts overflow during coin selection",
            )
        })?;
    if total_available < total_payments {
        return Err(BuilderError::new(
            "INSUFFICIENT_FUNDS",
            &format!(
                "Total available {} sats < total payments {} sats",
                total_available, total_payments
            ),
        ));
    }

    // -- Strategy 1: Try single UTXO exact/near-exact match (no change) --
    let mut best: Option<Candidate> = None;

    if max >= 1 {
        for &(idx, utxo, _, _) in &candidates {
            let utxo_refs = vec![utxo];
            let score = compute_score(&utxo_refs, utxo.value_sats, total_payments, fee_rate, &ow);

            if let Some(score) = score {
                let candidate = Candidate {
                    indices: vec![idx],
                    total_value: utxo.value_sats,
                    score,
                    strategy: Strategy::SingleUtxo,
                };

                if best
                    .as_ref()
                    .map(|b| candidate.score.better_than(&b.score))
                    .unwrap_or(true)
                {
                    best = Some(candidate);
                }
            }
        }
    }

    // -- Strategy 2: Branch-and-bound for exact match (no change output) --
    // Only try this for reasonable UTXO counts to avoid exponential blowup
    if candidates.len() <= MAX_BNB_UTXOS {
        let bnb_result = branch_and_bound(&candidates, total_payments, fee_rate, &ow, max);

        if let Some(bnb) = bnb_result {
            if best
                .as_ref()
                .map(|b| bnb.score.better_than(&b.score))
                .unwrap_or(true)
            {
                best = Some(bnb);
            }
        }
    }

    // -- Strategy 3: CoinGrinder — find minimum-weight input set with change --
    let cg_result = coin_grinder(&candidates, total_payments, fee_rate, &ow, max);

    if let Some(cg) = cg_result {
        if best
            .as_ref()
            .map(|b| cg.score.better_than(&b.score))
            .unwrap_or(true)
        {
            best = Some(cg);
        }
    }

    // -- Strategy 4: Greedy fallback (largest-first until funded) --
    let greedy = greedy_selection(&candidates, total_payments, fee_rate, &ow, max);

    if let Some(g) = greedy {
        if best
            .as_ref()
            .map(|b| g.score.better_than(&b.score))
            .unwrap_or(true)
        {
            best = Some(g);
        }
    }

    match best {
        Some(candidate) => {
            let selected: Vec<Utxo> = candidate
                .indices
                .iter()
                .map(|&i| utxos[i].clone())
                .collect();
            Ok(CoinSelection {
                selected,
                total_value: candidate.total_value,
                strategy: candidate.strategy,
            })
        }
        None => Err(BuilderError::new(
            "INSUFFICIENT_FUNDS",
            &format!(
                "Cannot fund transaction from {} of {} UTXOs: need {} sats + fee",
                max_inputs.unwrap_or(candidates.len() as u32),
                candidates.len(),
                total_payments
            ),
        )),
    }
}

/// Compute candidate score for a given set of selected UTXOs.
/// Returns None when the selection cannot fund the transaction.
fn compute_score(
    utxos: &[&Utxo],
    total_value: u64,
    total_payments: u64,
    fee_rate: f64,
    ow: &OutputWeights,
) -> Option<CandidateScore> {
    let input_types: Vec<ScriptType> = utxos.iter().map(|u| u.script_type).collect();

    // Check with change first
    let vbytes_with_change = ow.vbytes_with_change(&input_types);
    let fee_with_change = (vbytes_with_change as f64 * fee_rate).ceil() as u64;
    let needed_with_change = needed_amount(total_payments, fee_with_change)?;

    if total_value >= needed_with_change {
        let change = total_value - needed_with_change;
        if change >= DUST_THRESHOLD {
            return Some(CandidateScore {
                fee_sats: fee_with_change,
                input_count: utxos.len(),
                change_sats: change,
            });
        }
    }

    // Without change — all leftover becomes fee
    let vbytes_no_change = ow.vbytes_no_change(&input_types);
    let fee_no_change = (vbytes_no_change as f64 * fee_rate).ceil() as u64;
    if total_value >= needed_amount(total_payments, fee_no_change)? {
        let fee = total_value - total_payments;
        return Some(CandidateScore {
            fee_sats: fee,
            input_count: utxos.len(),
            change_sats: 0,
        });
    }

    None
}

/// Branch-and-bound search for exact match (no change output needed)
/// Looks for combinations where leftover is < cost_of_change (so dropping change is optimal)
fn branch_and_bound(
    candidates: &[(usize, &Utxo, u64, i64)],
    total_payments: u64,
    fee_rate: f64,
    ow: &OutputWeights,
    max_inputs: usize,
) -> Option<Candidate> {
    // Cost of creating a change output (the fee cost of the extra output weight)
    let cost_of_change = (ow.change_output_weight as f64 * fee_rate / 4.0).ceil() as u64;

    // We look for solutions where excess < cost_of_change (exact enough to skip change)
    let mut best: Option<Candidate> = None;
    let mut tries = 0u32;

    // Use DFS with pruning
    fn search(
        candidates: &[(usize, &Utxo, u64, i64)],
        pos: usize,
        current_indices: &mut Vec<usize>,
        current_value: u64,
        current_input_types: &mut Vec<ScriptType>,
        total_payments: u64,
        fee_rate: f64,
        ow: &OutputWeights,
        cost_of_change: u64,
        max_inputs: usize,
        best: &mut Option<Candidate>,
        tries: &mut u32,
    ) {
        *tries += 1;
        if *tries > MAX_SEARCH_TRIES {
            return;
        }

        if current_indices.len() > max_inputs {
            return;
        }

        if !current_indices.is_empty() {
            // Compute fee for current selection without change
            let vbytes = ow.vbytes_no_change(current_input_types);
            let fee = (vbytes as f64 * fee_rate).ceil() as u64;
            let Some(needed) = needed_amount(total_payments, fee) else {
                return;
            };

            if current_value >= needed {
                let excess = current_value - needed;
                // Accept if excess < cost_of_change (better to skip change)
                if excess < cost_of_change + DUST_THRESHOLD {
                    let total_fee = fee + excess;
                    let score = CandidateScore {
                        fee_sats: total_fee,
                        input_count: current_indices.len(),
                        change_sats: 0,
                    };
                    if best
                        .as_ref()
                        .map(|b| score.better_than(&b.score))
                        .unwrap_or(true)
                    {
                        *best = Some(Candidate {
                            indices: current_indices.clone(),
                            total_value: current_value,
                            score,
                            strategy: Strategy::BranchAndBound,
                        });
                    }
                    // Don't add more inputs — we already have enough
                    return;
                }
                // excess > cost_of_change — change would be better, don't explore further
                // (CoinGrinder will handle the change case)
                return;
            }

            // Check if remaining UTXOs can fill the gap
            let remaining_value: u64 = candidates[pos..]
                .iter()
                .map(|(_, u, _, _)| u.value_sats)
                .sum();
            if current_value
                .checked_add(remaining_value)
                .is_none_or(|v| v < needed)
            {
                return; // Prune: can't reach target
            }
        }

        for i in pos..candidates.len() {
            if *tries > MAX_SEARCH_TRIES {
                return;
            }
            if current_indices.len() >= max_inputs {
                return;
            }

            let (idx, utxo, _, eff_val) = &candidates[i];

            // Skip uneconomical UTXOs
            if *eff_val <= 0 {
                continue;
            }

            current_indices.push(*idx);
            current_input_types.push(utxo.script_type);
            let Some(new_value) = current_value.checked_add(utxo.value_sats) else {
                continue;
            };

            search(
                candidates,
                i + 1,
                current_indices,
                new_value,
                current_input_types,
                total_payments,
                fee_rate,
                ow,
                cost_of_change,
                max_inputs,
                best,
                tries,
            );

            current_indices.pop();
            current_input_types.pop();
        }
    }

    let mut indices = Vec::new();
    let mut input_types = Vec::new();
    search(
        candidates,
        0,
        &mut indices,
        0,
        &mut input_types,
        total_payments,
        fee_rate,
        ow,
        cost_of_change,
        max_inputs,
        &mut best,
        &mut tries,
    );

    best
}

/// CoinGrinder: DFS branch-and-bound searching for minimum-weight input set
/// that can fund the transaction WITH change (non-dust change output).
///
/// Inspired by Bitcoin Core's CoinGrinder algorithm:
/// - Sort candidates by effective value descending (weight as tiebreaker)
/// - Target: enough to produce non-dust change after paying fee_with_change
/// - Prune branches where current_weight >= best_weight
/// - Prune branches where remaining UTXOs' values can't reach target
/// - 100,000 iteration cap
/// - Returns the minimum-fee solution with change
fn coin_grinder(
    candidates: &[(usize, &Utxo, u64, i64)],
    total_payments: u64,
    fee_rate: f64,
    ow: &OutputWeights,
    max_inputs: usize,
) -> Option<Candidate> {
    if candidates.is_empty() || max_inputs == 0 {
        return None;
    }

    // CoinGrinder sorts by effective value descending, weight ascending as tiebreaker.
    // Our candidates are already sorted by effective value descending.
    // Re-sort with weight tiebreaker for CoinGrinder.
    let mut cg_candidates: Vec<(usize, &Utxo, u64, i64)> = candidates.to_vec();
    cg_candidates.sort_by(|a, b| {
        b.3.cmp(&a.3) // effective value descending
            .then(a.2.cmp(&b.2)) // weight ascending (prefer lighter at same eff value)
    });

    // Pre-compute suffix sums of raw values for pruning
    let n = cg_candidates.len();
    let mut suffix_value = vec![0u64; n + 1];
    for i in (0..n).rev() {
        suffix_value[i] = suffix_value[i + 1].saturating_add(cg_candidates[i].1.value_sats);
    }

    let mut best: Option<Candidate> = None;
    let mut best_weight: u64 = u64::MAX;
    let mut tries: u32 = 0;

    // DFS state
    struct State {
        indices: Vec<usize>, // original UTXO indices
        input_types: Vec<ScriptType>,
        current_value: u64,
        current_weight: u64, // total input weight
    }

    fn search(
        cg_candidates: &[(usize, &Utxo, u64, i64)],
        suffix_value: &[u64],
        pos: usize,
        state: &mut State,
        total_payments: u64,
        fee_rate: f64,
        ow: &OutputWeights,
        max_inputs: usize,
        best: &mut Option<Candidate>,
        best_weight: &mut u64,
        tries: &mut u32,
    ) {
        *tries += 1;
        if *tries > MAX_SEARCH_TRIES {
            return;
        }

        if state.indices.len() > max_inputs {
            return;
        }

        // Prune: current weight already >= best weight, can't improve
        if state.current_weight >= *best_weight {
            return;
        }

        if !state.indices.is_empty() {
            // Compute fee WITH change for current selection
            let vbytes_with_change = ow.vbytes_with_change(&state.input_types);
            let fee_with_change = (vbytes_with_change as f64 * fee_rate).ceil() as u64;
            let Some(needed_with_change) = needed_amount(total_payments, fee_with_change)
                .and_then(|n| n.checked_add(DUST_THRESHOLD))
            else {
                return;
            };

            if state.current_value >= needed_with_change {
                let change_amount = state.current_value - total_payments - fee_with_change;
                if change_amount >= DUST_THRESHOLD {
                    // Valid solution with change!
                    let score = CandidateScore {
                        fee_sats: fee_with_change,
                        input_count: state.indices.len(),
                        change_sats: change_amount,
                    };
                    if state.current_weight < *best_weight
                        || (state.current_weight == *best_weight
                            && best
                                .as_ref()
                                .map(|b| score.better_than(&b.score))
                                .unwrap_or(true))
                    {
                        *best = Some(Candidate {
                            indices: state.indices.clone(),
                            total_value: state.current_value,
                            score,
                            strategy: Strategy::CoinGrinder,
                        });
                        *best_weight = state.current_weight;
                    }
                    // Don't return — there might be a solution with the same weight
                    // but fewer inputs at this level. Actually, adding more inputs
                    // always increases weight, so we can return.
                    return;
                }
            }

            // Prune: can remaining UTXOs reach the target (including min fee)?
            // We need: current_value + remaining >= total_payments + min_fee + DUST_THRESHOLD
            // Use fee_no_change (smaller) as a conservative lower bound for pruning.
            let vbytes_no_change = ow.vbytes_no_change(&state.input_types);
            let min_fee = (vbytes_no_change as f64 * fee_rate).ceil() as u64;
            let Some(min_needed) =
                needed_amount(total_payments, min_fee).and_then(|n| n.checked_add(DUST_THRESHOLD))
            else {
                return;
            };

            if state
                .current_value
                .checked_add(suffix_value[pos])
                .is_none_or(|v| v < min_needed)
            {
                return;
            }
        }

        for i in pos..cg_candidates.len() {
            if *tries > MAX_SEARCH_TRIES {
                return;
            }
            if state.indices.len() >= max_inputs {
                return;
            }

            let (idx, utxo, iw, eff_val) = &cg_candidates[i];

            // Skip uneconomical UTXOs
            if *eff_val <= 0 {
                continue;
            }

            // Prune: adding this UTXO's weight would exceed best
            if state.current_weight + iw > *best_weight {
                // Since remaining candidates at same or higher positions have
                // >= their weight, we can't improve. But weight varies, so
                // we only skip this specific candidate.
                continue;
            }

            state.indices.push(*idx);
            state.input_types.push(utxo.script_type);
            if let Some(new_value) = state.current_value.checked_add(utxo.value_sats) {
                state.current_value = new_value;
            } else {
                state.input_types.pop();
                state.indices.pop();
                continue;
            }
            state.current_weight += iw;

            search(
                cg_candidates,
                suffix_value,
                i + 1,
                state,
                total_payments,
                fee_rate,
                ow,
                max_inputs,
                best,
                best_weight,
                tries,
            );

            state.current_weight -= iw;
            state.current_value -= utxo.value_sats;
            state.input_types.pop();
            state.indices.pop();
        }
    }

    let mut state = State {
        indices: Vec::new(),
        input_types: Vec::new(),
        current_value: 0,
        current_weight: 0,
    };

    search(
        &cg_candidates,
        &suffix_value,
        0,
        &mut state,
        total_payments,
        fee_rate,
        ow,
        max_inputs,
        &mut best,
        &mut best_weight,
        &mut tries,
    );

    best
}

/// Classic greedy selection: add UTXOs (largest effective value first) until funded
fn greedy_selection(
    candidates: &[(usize, &Utxo, u64, i64)],
    total_payments: u64,
    fee_rate: f64,
    ow: &OutputWeights,
    max_inputs: usize,
) -> Option<Candidate> {
    let mut selected_indices: Vec<usize> = Vec::new();
    let mut selected_utxos: Vec<&Utxo> = Vec::new();
    let mut total_value: u64 = 0;

    for &(idx, utxo, _, _) in candidates {
        if selected_indices.len() >= max_inputs {
            break;
        }
        selected_indices.push(idx);
        selected_utxos.push(utxo);
        total_value += utxo.value_sats;

        let input_types: Vec<ScriptType> = selected_utxos.iter().map(|u| u.script_type).collect();

        // Check if we can fund without change
        let vbytes_no_change = ow.vbytes_no_change(&input_types);
        let fee_no_change = (vbytes_no_change as f64 * fee_rate).ceil() as u64;

        if total_value >= total_payments + fee_no_change {
            if let Some(score) =
                compute_score(&selected_utxos, total_value, total_payments, fee_rate, ow)
            {
                return Some(Candidate {
                    indices: selected_indices,
                    total_value,
                    score,
                    strategy: Strategy::Greedy,
                });
            }
        }
    }

    None
}
