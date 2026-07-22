import { useTranslation } from "react-i18next";

import type { GameAction, PlayerId, WaitingFor } from "../../adapter/types.ts";
import { useGameDispatch } from "../../hooks/useGameDispatch.ts";
import { useCanActForWaitingState } from "../../hooks/usePlayerId.ts";
import { useGameStore } from "../../stores/gameStore.ts";
import { getOpponentDisplayName } from "../../stores/multiplayerStore.ts";
import { ChoiceModal } from "./ChoiceModal.tsx";

type ZoneOpponentChooserWaitingFor = Extract<
  WaitingFor,
  { type: "ChooseFromZoneOpponentChooser" }
>;

interface ZoneOpponentChooserModalContentProps {
  waitingFor: ZoneOpponentChooserWaitingFor;
  seatOrder?: PlayerId[];
  dispatch: (action: GameAction) => void | Promise<void>;
}

/**
 * CR 608.2d: "An opponent chooses …" from a zone in a multiplayer game — the
 * controller decides which opponent makes the choice before the zone choice
 * itself is presented to that opponent (Plargg and Nassari's release notes:
 * "you choose which opponent gets to choose one of the exiled nonland cards").
 */
export function ZoneOpponentChooserModalContent({
  waitingFor,
  seatOrder,
  dispatch,
}: ZoneOpponentChooserModalContentProps) {
  const { t } = useTranslation("game");
  const candidates = [...waitingFor.data.candidates].sort((a, b) => {
    const aIdx = seatOrder?.indexOf(a) ?? a;
    const bIdx = seatOrder?.indexOf(b) ?? b;
    return aIdx - bIdx;
  });

  return (
    <ChoiceModal
      title={t("zoneOpponentChooser.title", "Choose Opponent")}
      subtitle={t(
        "zoneOpponentChooser.subtitle",
        "Choose which opponent makes the choice.",
      )}
      options={candidates.map((opponent) => ({
        id: String(opponent),
        label: getOpponentDisplayName(opponent),
      }))}
      onChoose={(id) => {
        dispatch({
          type: "ChooseZoneOpponentChooser",
          data: { opponent: Number(id) },
        });
      }}
    />
  );
}

export function ZoneOpponentChooserModal() {
  const canActForWaitingState = useCanActForWaitingState();
  const dispatch = useGameDispatch();
  const waitingFor = useGameStore((s) => s.waitingFor);
  const seatOrder = useGameStore((s) => s.gameState?.seat_order);

  if (waitingFor?.type !== "ChooseFromZoneOpponentChooser") return null;
  if (!canActForWaitingState) return null;

  return (
    <ZoneOpponentChooserModalContent
      waitingFor={waitingFor}
      seatOrder={seatOrder}
      dispatch={dispatch}
    />
  );
}
