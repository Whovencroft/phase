import { useState } from "react";

import type { DebugAction, Phase, PlayerId } from "../../adapter/types";
import {
  AccordionItem,
  FieldRow,
  PlayerSelect,
  SelectInput,
  SubmitButton,
  useAccordion,
} from "./debugFields";

const PHASES: readonly Phase[] = [
  "Untap",
  "Upkeep",
  "Draw",
  "PreCombatMain",
  "BeginCombat",
  "DeclareAttackers",
  "DeclareBlockers",
  "CombatDamage",
  "EndCombat",
  "PostCombatMain",
  "End",
  "Cleanup",
] as const;

interface Props {
  onDispatch: (action: DebugAction) => void;
}

function SetPhaseForm({ onDispatch }: Props) {
  const [phase, setPhase] = useState<Phase>("PreCombatMain");
  const [activePlayer, setActivePlayer] = useState<PlayerId>(0);

  return (
    <>
      <FieldRow label="Phase">
        <SelectInput value={phase} onChange={setPhase} options={PHASES} />
      </FieldRow>
      <FieldRow label="Active Player">
        <PlayerSelect value={activePlayer} onChange={setActivePlayer} />
      </FieldRow>
      <SubmitButton
        onClick={() =>
          onDispatch({ type: "SetPhase", data: { phase, active_player: activePlayer } })
        }
      >
        Set Phase
      </SubmitButton>
    </>
  );
}

export function DebugFlowActions({ onDispatch }: Props) {
  const { expanded, toggle } = useAccordion();

  return (
    <div>
      <AccordionItem label="Set Phase" expanded={expanded === "phase"} onToggle={() => toggle("phase")}>
        <SetPhaseForm onDispatch={onDispatch} />
      </AccordionItem>
      <AccordionItem label="Run SBAs" expanded={expanded === "sba"} onToggle={() => toggle("sba")}>
        <SubmitButton onClick={() => onDispatch({ type: "RunStateBasedActions" })}>
          Run State-Based Actions
        </SubmitButton>
      </AccordionItem>
    </div>
  );
}
