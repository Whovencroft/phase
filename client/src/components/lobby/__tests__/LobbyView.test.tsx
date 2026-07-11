import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/react";

import { LobbyView } from "../LobbyView";
import { useMultiplayerStore } from "../../../stores/multiplayerStore";

/**
 * LobbyView now delegates to the shared subscription socket via the
 * multiplayer store's `subscribeLobby` / `ensureSubscriptionSocket`
 * actions, rather than opening its own `WebSocket` directly. Tests stub
 * those store actions with promise-returning mocks so the component's
 * cleanup paths (offline fallback, unmount-before-subscribe) stay
 * observable without a real socket.
 */
describe("LobbyView", () => {
  const originalSubscribeLobby = useMultiplayerStore.getState().subscribeLobby;
  const originalEnsureSubscription =
    useMultiplayerStore.getState().ensureSubscriptionSocket;

  beforeEach(() => {
    useMultiplayerStore.setState({ serverAddress: "wss://us.phase-rs.dev/ws" });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    useMultiplayerStore.setState({
      subscribeLobby: originalSubscribeLobby,
      ensureSubscriptionSocket: originalEnsureSubscription,
    });
  });

  it("calls onServerOffline when the shared subscription socket is unreachable", async () => {
    // Store returns `null` when the socket can't be opened (withReconnect
    // exhausted, invalid URL, etc.). LobbyView's offline fallback fires.
    useMultiplayerStore.setState({
      subscribeLobby: vi.fn().mockResolvedValue(null),
      ensureSubscriptionSocket: vi.fn().mockResolvedValue(null),
    });
    const onServerOffline = vi.fn();
    render(
      <LobbyView
        onHostGame={vi.fn()}
        onHostP2P={vi.fn()}
        onJoinGame={vi.fn()}
        onServerOffline={onServerOffline}
      />,
    );

    // Flush the microtask from `await subscribeLobby()`.
    await Promise.resolve();
    await Promise.resolve();

    expect(onServerOffline).toHaveBeenCalledTimes(1);
  });

  it("does not call onServerOffline when component unmounts before subscribe resolves", async () => {
    // Mount, unmount, THEN resolve the pending subscribe — the effect's
    // `cancelled` guard must suppress the offline callback.
    let resolveSubscribe!: (v: null) => void;
    useMultiplayerStore.setState({
      subscribeLobby: vi
        .fn()
        .mockReturnValue(
          new Promise<null>((r) => {
            resolveSubscribe = r;
          }),
        ),
      ensureSubscriptionSocket: vi.fn().mockResolvedValue(null),
    });
    const onServerOffline = vi.fn();
    const { unmount } = render(
      <LobbyView
        onHostGame={vi.fn()}
        onHostP2P={vi.fn()}
        onJoinGame={vi.fn()}
        onServerOffline={onServerOffline}
      />,
    );

    unmount();
    resolveSubscribe(null);
    await Promise.resolve();

    expect(onServerOffline).not.toHaveBeenCalled();
  });

  it("does not subscribe in p2p mode", () => {
    const subscribeLobby = vi.fn();
    useMultiplayerStore.setState({
      subscribeLobby,
      ensureSubscriptionSocket: vi.fn(),
    });
    render(
      <LobbyView
        onHostGame={vi.fn()}
        onHostP2P={vi.fn()}
        onJoinGame={vi.fn()}
        connectionMode="p2p"
        onServerOffline={vi.fn()}
      />,
    );

    expect(subscribeLobby).not.toHaveBeenCalled();
  });

  it("fires offline fallback when the stored server address is invalid", async () => {
    useMultiplayerStore.setState({
      serverAddress: "wss:",
      // The real store's `ensureSubscriptionSocket` rejects invalid URLs
      // with `null`; mirror that contract here so the UI path is tested.
      subscribeLobby: vi.fn().mockResolvedValue(null),
      ensureSubscriptionSocket: vi.fn().mockResolvedValue(null),
    });
    const onServerOffline = vi.fn();

    render(
      <LobbyView
        onHostGame={vi.fn()}
        onHostP2P={vi.fn()}
        onJoinGame={vi.fn()}
        onServerOffline={onServerOffline}
      />,
    );

    await Promise.resolve();
    await Promise.resolve();

    expect(onServerOffline).toHaveBeenCalledTimes(1);
  });
});
