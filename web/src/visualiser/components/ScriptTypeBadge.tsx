import Tooltip from "./Tooltip";

const SCRIPT_TYPES: Record<string, { cls: string; tip: string }> = {
  p2pkh: {
    cls: "legacy",
    tip: "Pay-to-PubKey-Hash. Classic single-signature output.",
  },
  p2sh: {
    cls: "legacy",
    tip: "Pay-to-Script-Hash. Common for multisig or scripts.",
  },
  p2wpkh: {
    cls: "sw",
    tip: "SegWit single-signature output. Lower weight for signatures.",
  },
  p2wsh: {
    cls: "sw",
    tip: "SegWit script hash. Used for multisig or advanced scripts.",
  },
  "p2sh-p2wpkh": {
    cls: "sw",
    tip: "SegWit wrapped inside P2SH for older wallet compatibility.",
  },
  "p2sh-p2wsh": {
    cls: "sw",
    tip: "SegWit script hash wrapped in P2SH for compatibility.",
  },
  p2tr: {
    cls: "tr",
    tip: "Modern Taproot output. More private and flexible spending.",
  },
  p2tr_keypath: {
    cls: "tr",
    tip: "Taproot spend using a single key signature.",
  },
  p2tr_scriptpath: {
    cls: "tr",
    tip: "Taproot spend revealing a script path.",
  },
  op_return: {
    cls: "legacy",
    tip: "Carries arbitrary data. Not spendable as coins.",
  },
  unknown: {
    cls: "legacy",
    tip: "Unrecognized script pattern.",
  },
};

export default function ScriptTypeBadge({
  scriptType,
}: {
  scriptType: string;
}) {
  const info = SCRIPT_TYPES[scriptType] || {
    label: scriptType,
    cls: "legacy",
    tip: "Script type not recognized.",
  };
  return (
    <Tooltip tip={info.tip}>
      <span className={`badge badge-${info.cls}`}>{scriptType}</span>
    </Tooltip>
  );
}
