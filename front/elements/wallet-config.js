import { createConfig, connect, disconnect, getConnection, reconnect, injected, http } from "@wagmi/core";
import { mainnet, sepolia } from "viem/chains";

export const wagmiConfig = createConfig({
  chains: [mainnet, sepolia],
  transports: {
    [mainnet.id]: http(),
    [sepolia.id]: http(),
  },
  connectors: [injected()],
});

/** Call on dApp load to restore previous wallet connection from storage */
export function restoreWalletConnection() {
  return reconnect(wagmiConfig);
}

/**
 * Connect ETH wallet (injected provider). Resolves with address or throws.
 * @returns {Promise<string>} Connected account address
 */
export async function connectWallet() {
  const connection = getConnection(wagmiConfig);
  if (connection?.address) return connection.address;
  const result = await connect(wagmiConfig, { connector: injected() });
  const address = result?.address ?? result?.accounts?.[0];
  if (!address) throw new Error("No account returned");
  return address;
}

export async function disconnectWallet() {
  await disconnect(wagmiConfig);
}

export function getConnectedAddress() {
  const connection = getConnection(wagmiConfig);
  return connection?.address ?? null;
}
