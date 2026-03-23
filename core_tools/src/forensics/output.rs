use crate::forensics::types::*;
use std::io::Write;
use std::time::{Duration, UNIX_EPOCH};

/// Generate a Markdown report for a block file analysis
pub fn generate_markdown_report(output: &FileOutput) -> String {
    let mut md = String::new();

    // Title
    md.push_str(&format!("# Chain Analysis Report: {}\n\n", output.file));

    // File overview
    md.push_str("## Summary\n\n");
    md.push_str("| Property | Value |\n");
    md.push_str("|----------|-------|\n");
    md.push_str(&format!("| Source File | `{}` |\n", output.file));
    md.push_str(&format!("| Blocks Analyzed | {} |\n", output.block_count));
    md.push_str(&format!(
        "| Total Transactions | {} |\n",
        output.analysis_summary.total_transactions_analyzed
    ));
    md.push_str(&format!(
        "| Flagged Transactions | {} |\n",
        output.analysis_summary.flagged_transactions
    ));
    md.push_str(&format!(
        "| Heuristics Applied | {} |\n",
        output.analysis_summary.heuristics_applied.join(", ")
    ));
    md.push('\n');

    // Fee Rate distribution
    md.push_str("## Fee Rate Statistics\n\n");
    let fee = &output.analysis_summary.fee_rate_stats;
    md.push_str("| Metric | Value (sat/vB) |\n");
    md.push_str("|--------|----------------|\n");
    md.push_str(&format!("| Minimum | {:.1} |\n", fee.min_sat_vb));
    md.push_str(&format!("| Maximum | {:.1} |\n", fee.max_sat_vb));
    md.push_str(&format!("| Median | {:.1} |\n", fee.median_sat_vb));
    md.push_str(&format!("| Mean | {:.1} |\n", fee.mean_sat_vb));
    md.push('\n');

    // Script type breakdown
    md.push_str("## Script Type Distribution\n\n");
    md.push_str("| Script Type | Count |\n");
    md.push_str("|-------------|-------|\n");
    let mut script_entries: Vec<_> = output
        .analysis_summary
        .script_type_distribution
        .iter()
        .collect();
    script_entries.sort_by(|a, b| b.1.cmp(a.1));
    for (script_type, count) in &script_entries {
        md.push_str(&format!("| {} | {} |\n", script_type, count));
    }
    md.push('\n');

    // Per-block sections
    for (block_idx, block) in output.blocks.iter().enumerate() {
        md.push_str(&format!(
            "---\n\n## Block {} — Height {}\n\n",
            block_idx + 1,
            block.block_height
        ));
        md.push_str(&format!("- **Block Hash:** `{}`\n", block.block_hash));
        md.push_str(&format!(
            "- **Timestamp:** {}\n",
            format_block_timestamp(block.block_timestamp)
        ));
        md.push_str(&format!("- **Transaction Count:** {}\n", block.tx_count));
        md.push_str(&format!(
            "- **Flagged Transactions:** {}\n",
            block.analysis_summary.flagged_transactions
        ));
        md.push('\n');

        // Block-level fee rate stats
        md.push_str("### Fee Rate Statistics\n\n");
        let bfee = &block.analysis_summary.fee_rate_stats;
        md.push_str("| Metric | Value (sat/vB) |\n");
        md.push_str("|--------|----------------|\n");
        md.push_str(&format!("| Minimum | {:.1} |\n", bfee.min_sat_vb));
        md.push_str(&format!("| Maximum | {:.1} |\n", bfee.max_sat_vb));
        md.push_str(&format!("| Median | {:.1} |\n", bfee.median_sat_vb));
        md.push_str(&format!("| Mean | {:.1} |\n", bfee.mean_sat_vb));
        md.push('\n');

        // Heuristic findings
        md.push_str("### Heuristic Findings\n\n");
        md.push_str("| Heuristic | Transactions Flagged |\n");
        md.push_str("|-----------|---------------------|\n");
        let mut heur_entries: Vec<_> = block.heuristic_findings.iter().collect();
        heur_entries.sort_by(|a, b| b.1.cmp(a.1));
        for (hid, count) in &heur_entries {
            md.push_str(&format!("| {} | {} |\n", format_heuristic_name(hid), count));
        }
        if heur_entries.is_empty() {
            md.push_str("| None | 0 |\n");
        }
        md.push('\n');

        // Transaction classification distribution
        md.push_str("### Transaction Classifications\n\n");
        md.push_str("| Classification | Count |\n");
        md.push_str("|----------------|-------|\n");
        let mut class_entries: Vec<_> = block.classification_counts.iter().collect();
        class_entries.sort_by(|a, b| b.1.cmp(a.1));
        for (class, count) in &class_entries {
            md.push_str(&format!("| {} | {} |\n", class, count));
        }
        if class_entries.is_empty() {
            md.push_str("| unknown | 0 |\n");
        }
        md.push('\n');

        // Notable transactions
        md.push_str("### Notable Transactions\n\n");
        let mut notable_count = 0;
        for tx in &block.notable_transactions {
            md.push_str(&format!("#### `{}`\n\n", &tx.txid[..16]));
            md.push_str(&format!("- **Full TXID:** `{}`\n", tx.txid));
            md.push_str(&format!("- **Classification:** {}\n", tx.classification));
            md.push_str("- **Heuristics Fired:**\n");
            for result in &tx.heuristics {
                let mut detail = format!("  - **{}**", format_heuristic_name(&result.id));
                if let Some(ref conf) = result.confidence {
                    detail.push_str(&format!(" (confidence: {})", conf));
                }
                if let Some(ref method) = result.method {
                    detail.push_str(&format!(" — method: {}", method));
                }
                if let Some(ref details) = result.details {
                    detail.push_str(&format!(" — {}", details));
                }
                md.push_str(&format!("{}\n", detail));
            }
            md.push('\n');
            notable_count += 1;
        }

        if notable_count == 0 {
            md.push_str("No particularly notable transactions (coinjoin, consolidation, etc.) detected in this block.\n\n");
        }

        // Script type distribution for this block
        md.push_str("### Script Type Distribution\n\n");
        md.push_str("| Script Type | Count |\n");
        md.push_str("|-------------|-------|\n");
        let mut block_script: Vec<_> = block
            .analysis_summary
            .script_type_distribution
            .iter()
            .collect();
        block_script.sort_by(|a, b| b.1.cmp(a.1));
        for (stype, count) in &block_script {
            md.push_str(&format!("| {} | {} |\n", stype, count));
        }
        md.push('\n');
    }

    // Footer
    md.push_str("---\n\n");
    md.push_str("*Report generated by Sherlock Chain Analysis Engine*\n");

    md
}

fn format_block_timestamp(timestamp: u32) -> String {
    let dt = UNIX_EPOCH + Duration::from_secs(timestamp as u64);
    match dt.duration_since(UNIX_EPOCH) {
        Ok(duration) => format!("{} (unix)", duration.as_secs()),
        Err(_) => format!("{} (unix)", timestamp),
    }
}

/// Format heuristic ID to human-readable name
fn format_heuristic_name(id: &str) -> &str {
    match id {
        "cioh" => "Common Input Ownership (CIOH)",
        "change_detection" => "Change Detection",
        "address_reuse" => "Address Reuse",
        "coinjoin" => "CoinJoin Detection",
        "consolidation" => "Consolidation Detection",
        "self_transfer" => "Self-Transfer Detection",
        "peeling_chain" => "Peeling Chain Detection",
        "op_return" => "OP_RETURN Analysis",
        "round_number_payment" => "Round Number Payment",
        _ => id,
    }
}

/// Write the JSON output to a file
pub fn write_json_output(
    output: &FileOutput,
    path: &str,
) -> Result<(), crate::forensics::error::AppError> {
    let json = serde_json::to_string_pretty(output)?;
    let mut file = std::fs::File::create(path)?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

/// Write the Markdown report to a file
pub fn write_markdown_report(
    output: &FileOutput,
    path: &str,
) -> Result<(), crate::forensics::error::AppError> {
    let md = generate_markdown_report(output);
    let mut file = std::fs::File::create(path)?;
    file.write_all(md.as_bytes())?;
    Ok(())
}
