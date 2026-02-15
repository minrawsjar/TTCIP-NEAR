import { initiateDeveloperControlledWalletsClient } from "@circle-fin/developer-controlled-wallets";
import { v4 as uuidv4 } from "uuid";

// ============================================================================
// Circle Developer-Controlled Wallet Service
// Creates and manages wallets on Arc Testnet for SMS users
// ============================================================================

export interface CircleWallet {
  id: string;
  address: string;
  blockchain: string;
  state: string;
}

export interface WalletBalance {
  tokenId: string;
  amount: string;
  token: {
    symbol: string;
    name: string;
    decimals: number;
  };
}

export class CircleWalletService {
  private client: ReturnType<typeof initiateDeveloperControlledWalletsClient>;
  private walletSetId: string | null = null;

  constructor() {
    const apiKey = process.env.CIRCLE_API_KEY;
    const entitySecret = process.env.CIRCLE_ENTITY_SECRET;

    if (!apiKey || !entitySecret) {
      throw new Error(
        "CIRCLE_API_KEY and CIRCLE_ENTITY_SECRET must be set in environment"
      );
    }

    this.client = initiateDeveloperControlledWalletsClient({
      apiKey,
      entitySecret,
    });
  }

  // ============================================================================
  // Initialize: Create or retrieve the wallet set for TextChain users
  // ============================================================================
  async initialize(): Promise<void> {
    try {
      // Try to get existing wallet sets
      const listResponse = await this.client.listWalletSets({});
      const walletSets = listResponse.data?.walletSets;

      if (walletSets && walletSets.length > 0) {
        // Use existing "TextChain Users" wallet set if found
        const existing = walletSets.find(
          (ws: any) => ws.name === "TextChain Users"
        );
        if (existing) {
          this.walletSetId = existing.id;
          console.log(`✅ Using existing wallet set: ${this.walletSetId}`);
          return;
        }
      }

      // Create new wallet set
      const response = await this.client.createWalletSet({
        name: "TextChain Users",
      });

      this.walletSetId = response.data?.walletSet?.id ?? null;
      console.log(`✅ Created wallet set: ${this.walletSetId}`);
    } catch (error: any) {
      console.error("❌ Failed to initialize wallet set:", error.message);
      throw error;
    }
  }

  // ============================================================================
  // Create a new wallet on Arc Testnet for a user
  // ============================================================================
  async createWallet(): Promise<CircleWallet> {
    if (!this.walletSetId) {
      await this.initialize();
    }

    try {
      const response = await this.client.createWallets({
        blockchains: ["ARC-TESTNET" as any],
        count: 1,
        walletSetId: this.walletSetId!,
      });

      const wallet = response.data?.wallets?.[0];
      if (!wallet) {
        throw new Error("No wallet returned from Circle API");
      }

      console.log(
        `✅ Created Arc wallet: ${wallet.address} (id: ${wallet.id})`
      );

      return {
        id: wallet.id,
        address: wallet.address ?? "",
        blockchain: wallet.blockchain ?? "ARC-TESTNET",
        state: wallet.state ?? "LIVE",
      };
    } catch (error: any) {
      console.error("❌ Failed to create wallet:", error.message);
      throw error;
    }
  }

  // ============================================================================
  // Get wallet by ID
  // ============================================================================
  async getWallet(walletId: string): Promise<CircleWallet | null> {
    try {
      const response = await this.client.getWallet({ id: walletId });
      const wallet = response.data?.wallet;

      if (!wallet) return null;

      return {
        id: wallet.id,
        address: wallet.address ?? "",
        blockchain: wallet.blockchain ?? "",
        state: wallet.state ?? "",
      };
    } catch (error: any) {
      console.error(`❌ Failed to get wallet ${walletId}:`, error.message);
      return null;
    }
  }

  // ============================================================================
  // Get wallet balances (USDC, ETH, etc.)
  // ============================================================================
  async getBalances(walletId: string): Promise<WalletBalance[]> {
    try {
      const response = await (this.client as any).listWalletBalance({
        id: walletId,
      });

      const tokenBalances = response.data?.tokenBalances ?? [];

      return tokenBalances.map((tb: any) => ({
        tokenId: tb.token?.id ?? "",
        amount: tb.amount ?? "0",
        token: {
          symbol: tb.token?.symbol ?? "UNKNOWN",
          name: tb.token?.name ?? "Unknown Token",
          decimals: tb.token?.decimals ?? 18,
        },
      }));
    } catch (error: any) {
      console.error(
        `❌ Failed to get balances for ${walletId}:`,
        error.message
      );
      return [];
    }
  }

  // ============================================================================
  // Get USDC balance specifically
  // ============================================================================
  async getUsdcBalance(walletId: string): Promise<string> {
    const balances = await this.getBalances(walletId);
    const usdc = balances.find(
      (b) =>
        b.token.symbol === "USDC" ||
        b.token.symbol === "usdc" ||
        b.token.name.toLowerCase().includes("usd coin")
    );
    return usdc?.amount ?? "0";
  }

  // ============================================================================
  // Transfer USDC between Circle wallets on Arc
  // ============================================================================
  async transferUsdc(
    fromWalletId: string,
    toAddress: string,
    amount: string,
    usdcTokenId: string
  ): Promise<{ transactionId: string; state: string }> {
    try {
      const response = await this.client.createTransaction({
        amount: [amount],
        destinationAddress: toAddress,
        tokenId: usdcTokenId,
        walletId: fromWalletId,
        fee: {
          type: "level",
          config: {
            feeLevel: "HIGH",
          },
        },
      });

      const txId = (response.data as any)?.id ?? "unknown";
      const state = (response.data as any)?.state ?? "INITIATED";

      console.log(`✅ USDC transfer initiated: ${txId} (${state})`);

      return { transactionId: txId, state };
    } catch (error: any) {
      console.error("❌ USDC transfer failed:", error.message);
      throw error;
    }
  }

  // ============================================================================
  // Get the underlying Circle SDK client (for Bridge Kit integration)
  // ============================================================================
  getClient() {
    return this.client;
  }

  getWalletSetId(): string | null {
    return this.walletSetId;
  }
}
