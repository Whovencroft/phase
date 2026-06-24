import { useNavigate } from "react-router";

import { useAudioContext } from "../audio/useAudioContext";
import { ScreenChrome } from "../components/chrome/ScreenChrome";
import { CardCoverageDashboard } from "../components/controls/CardCoverageDashboard";
import { MenuParticles } from "../components/menu/MenuParticles";

export function CoveragePage() {
  const navigate = useNavigate();
  useAudioContext("menu");

  return (
    <div className="menu-scene relative flex min-h-screen flex-col overflow-hidden">
      <MenuParticles />
      <ScreenChrome onBack={() => navigate("/")} />
      <div className="menu-scene__vignette" />
      <div className="menu-scene__sigil menu-scene__sigil--left" />
      <div className="menu-scene__sigil menu-scene__sigil--right" />
      <div className="menu-scene__haze" />
      <div className="relative z-10 flex min-h-0 flex-1 flex-col px-2 pt-16 pb-4 sm:px-6 sm:pt-20 sm:pb-6 lg:px-8">
        <CardCoverageDashboard />
      </div>
    </div>
  );
}
