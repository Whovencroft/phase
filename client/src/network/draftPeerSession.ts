/**
 * Draft-specific PeerSession wrapper.
 *
 * Wraps `createPeerSession` from `peer.ts` to decode/encode
 * `DraftP2PMessage` instead of `P2PMessage`. Shares the same
 * DataConnection transport — the only difference is which message
 * union is deserialized.
 */

import type { DataConnection } from "peerjs";

import type { DraftP2PMessage } from "./draftProtocol";
import { decodeDraftWireMessage, encodeDraftWireMessage } from "./draftProtocol";

export interface DraftPeerSession {
  send(msg: DraftP2PMessage): Promise<void>;
  onMessage(handler: (msg: DraftP2PMessage) => void | Promise<void>): () => void;
  onDisconnect(handler: (reason: string) => void): () => void;
  close(reason?: string): void;
}

export interface DraftPeerSessionOptions {
  onSessionEnd?: () => void;
}

export function createDraftPeerSession(
  conn: DataConnection,
  options: DraftPeerSessionOptions = {},
): DraftPeerSession {
  const { onSessionEnd } = options;
  const messageHandlers = new Set<(msg: DraftP2PMessage) => void | Promise<void>>();
  const disconnectHandlers = new Set<(reason: string) => void>();
  let closed = false;

  // FIFO send queue for async compression
  let sendChain = Promise.resolve();

  function fireDisconnect(reason: string): void {
    if (closed) return;
    closed = true;
    for (const handler of disconnectHandlers) {
      try { handler(reason); } catch { /* best-effort */ }
    }
    disconnectHandlers.clear();
    messageHandlers.clear();
    onSessionEnd?.();
  }

  conn.on("data", (raw: unknown) => {
    if (closed) return;
    if (raw instanceof ArrayBuffer || raw instanceof Uint8Array) {
      const bytes = raw instanceof Uint8Array ? raw : new Uint8Array(raw);
      void decodeDraftWireMessage(bytes)
        .then((msg) => {
          for (const handler of messageHandlers) {
            handler(msg);
          }
        })
        .catch((err) => {
          console.warn("[DraftPeerSession] decode error:", err);
        });
    }
  });

  conn.on("close", () => fireDisconnect("connection closed"));
  conn.on("error", (err: Error) => fireDisconnect(err.message));

  const session: DraftPeerSession = {
    send(msg: DraftP2PMessage): Promise<void> {
      const p = sendChain.then(async () => {
        if (closed || !conn.open) return;
        const bytes = await encodeDraftWireMessage(msg);
        conn.send(bytes);
      });
      sendChain = p.catch(() => { /* swallow */ });
      return p;
    },
    onMessage(handler) {
      messageHandlers.add(handler);
      return () => { messageHandlers.delete(handler); };
    },
    onDisconnect(handler) {
      disconnectHandlers.add(handler);
      return () => { disconnectHandlers.delete(handler); };
    },
    close(reason?: string) {
      if (closed) return;
      fireDisconnect(reason ?? "closed");
      try { conn.close(); } catch { /* best-effort */ }
    },
  };

  return session;
}
