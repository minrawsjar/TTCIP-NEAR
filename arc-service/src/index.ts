import express from "express";
import dotenv from "dotenv";
import fs from "fs";
import path from "path";
import { CircleWalletService } from "./circle-wallet";
import { CashoutService } from "./cashout-service";

dotenv.config();

const app = express();
app.use(express.json());

// ============================================================================
// Persistent store: phone → Circle wallet mapping (JSON file)
// Survives container restarts
// ============================================================================
const WALLETS_FILE = path.join(__dirname, "..", "wallets.json");
const userWallets: Map<
  string,
  { walletId: string; address: string; blockchain: string }
> = new Map();

function loadWallets() {
  try {
    if (fs.existsSync(WALLETS_FILE)) {
      const data = JSON.parse(fs.readFileSync(WALLETS_FILE, "utf8"));
      for (const [phone, wallet] of Object.entries(data)) {
        userWallets.set(phone, wallet as any);
      }
      console.log(`📂 Loaded ${userWallets.size} wallet mappings from disk`);
    }
  } catch (e: any) {
    console.error("⚠️  Failed to load wallets file:", e.message);
  }
}

function saveWallets() {
  try {
    const data: Record<string, any> = {};
    for (const [phone, wallet] of userWallets.entries()) {
      data[phone] = wallet;
    }
    fs.writeFileSync(WALLETS_FILE, JSON.stringify(data, null, 2));
  } catch (e: any) {
    console.error("⚠️  Failed to save wallets file:", e.message);
  }
}

// Load persisted wallets on startup
loadWallets();

let walletService: CircleWalletService;
let cashoutService: CashoutService;

// ============================================================================
// Health check
// ============================================================================
app.get("/health", (_req, res) => {
  res.json({
    status: "ok",
    service: "arc-service",
    circle: !!process.env.CIRCLE_API_KEY,
    sepolia: !!process.env.PRIVATE_KEY,
  });
});

// ============================================================================
// POST /api/arc/wallet — Create or get Circle Wallet on Arc Testnet
// Body: { phone: string }
// ============================================================================
app.post("/api/arc/wallet", async (req, res) => {
  try {
    const { phone } = req.body;

    if (!phone) {
      return res.status(400).json({ success: false, error: "Missing phone" });
    }

    // Check if user already has a wallet
    const existing = userWallets.get(phone);
    if (existing) {
      console.log(`📱 Returning existing Arc wallet for ${phone}`);
      return res.json({
        success: true,
        wallet: existing,
        isNew: false,
      });
    }

    // Create new wallet on Arc Testnet
    console.log(`📱 Creating new Arc wallet for ${phone}...`);
    const wallet = await walletService.createWallet();

    const walletInfo = {
      walletId: wallet.id,
      address: wallet.address,
      blockchain: wallet.blockchain,
    };

    userWallets.set(phone, walletInfo);
    saveWallets();

    res.json({
      success: true,
      wallet: walletInfo,
      isNew: true,
    });
  } catch (error: any) {
    console.error("❌ Wallet creation error:", error.message);
    res.status(500).json({ success: false, error: error.message });
  }
});

// ============================================================================
// GET /api/arc/balance/:phone — Get USDC balance on Arc Testnet
// ============================================================================
app.get("/api/arc/balance/:phone", async (req, res) => {
  try {
    const { phone } = req.params;

    const walletInfo = userWallets.get(phone);
    if (!walletInfo) {
      return res.status(404).json({
        success: false,
        error: "No Arc wallet found. Send CASHOUT first to create one.",
      });
    }

    const balances = await walletService.getBalances(walletInfo.walletId);
    const usdcBalance = await walletService.getUsdcBalance(walletInfo.walletId);

    res.json({
      success: true,
      phone,
      arcWallet: walletInfo.address,
      usdcBalance,
      allBalances: balances,
    });
  } catch (error: any) {
    console.error("❌ Balance error:", error.message);
    res.status(500).json({ success: false, error: error.message });
  }
});

// ============================================================================
// POST /api/arc/quote — Get cashout quote (TXTC → USDC estimate)
// Body: { txtcAmount: string }
// ============================================================================
app.post("/api/arc/quote", async (req, res) => {
  try {
    const { txtcAmount } = req.body;

    if (!txtcAmount) {
      return res
        .status(400)
        .json({ success: false, error: "Missing txtcAmount" });
    }

    const quote = await cashoutService.getQuote(txtcAmount);

    res.json({
      success: true,
      quote,
    });
  } catch (error: any) {
    console.error("❌ Quote error:", error.message);
    res.status(500).json({ success: false, error: error.message });
  }
});

// ============================================================================
// POST /api/arc/cashout — Full cashout: TXTC → WETH → USDC on Arc
// Body: { phone: string, userAddress: string, txtcAmount: string }
// ============================================================================
app.post("/api/arc/cashout", async (req, res) => {
  try {
    const { phone, userAddress, txtcAmount, token } = req.body;
    const tokenType = (token || "TXTC").toUpperCase();

    if (!phone || !userAddress || !txtcAmount) {
      return res.status(400).json({
        success: false,
        error: "Missing phone, userAddress, or txtcAmount",
      });
    }

    console.log(
      `\n💰 Cashout request: ${txtcAmount} ${tokenType} from ${phone}`
    );

    // Step 1: Ensure user has an Arc wallet
    let walletInfo = userWallets.get(phone);
    if (!walletInfo) {
      console.log("   Creating Arc wallet for user...");
      const wallet = await walletService.createWallet();
      walletInfo = {
        walletId: wallet.id,
        address: wallet.address,
        blockchain: wallet.blockchain,
      };
      userWallets.set(phone, walletInfo);
      saveWallets();
    }

    // Step 2: Respond immediately (async processing for SMS timeout)
    res.json({
      success: true,
      message: "Cashout initiated",
      arcWallet: walletInfo.address,
    });

    // Step 3: Process cashout asynchronously
    (async () => {
      try {
        let result;
        if (tokenType === "ETH") {
          result = await cashoutService.cashoutEth(
            txtcAmount,
            userAddress,
            walletInfo!.walletId,
            walletInfo!.address
          );
        } else {
          result = await cashoutService.cashout(
            txtcAmount,
            userAddress,
            walletInfo!.walletId,
            walletInfo!.address
          );
        }

        if (result.success) {
          const label = tokenType === "ETH"
            ? `${result.txtcAmount} ETH`
            : `${result.txtcAmount} TXTC`;
          console.log(
            `\n✅ Cashout complete for ${phone}: ${label} → ~$${result.usdcEstimate} USDC`
          );

          // Notify backend to send SMS
          try {
            const backendUrl =
              process.env.BACKEND_URL || "http://localhost:3000";
            await fetch(`${backendUrl}/api/arc/notify`, {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                phone,
                message: `✅ Cashout complete!\n\n${label} → ~$${result.usdcEstimate} USDC\n\nArc Wallet: ${result.arcWalletAddress.slice(0, 10)}...\n\nReply BALANCE to check.`,
              }),
            });
          } catch (notifyError: any) {
            console.error(
              "⚠️  Failed to send notification:",
              notifyError.message
            );
          }
        } else {
          console.error(`❌ Cashout failed for ${phone}: ${result.error}`);
        }
      } catch (asyncError: any) {
        console.error("❌ Async cashout error:", asyncError.message);
      }
    })();
  } catch (error: any) {
    console.error("❌ Cashout initiation error:", error.message);
    res.status(500).json({ success: false, error: error.message });
  }
});

// ============================================================================
// POST /api/arc/pay — Send USDC to another user on Arc
// Body: { fromPhone: string, toPhone: string, amount: string }
// ============================================================================
app.post("/api/arc/pay", async (req, res) => {
  try {
    const { fromPhone, toPhone, amount } = req.body;

    if (!fromPhone || !toPhone || !amount) {
      return res.status(400).json({
        success: false,
        error: "Missing fromPhone, toPhone, or amount",
      });
    }

    // Get sender wallet
    const senderWallet = userWallets.get(fromPhone);
    if (!senderWallet) {
      return res.status(404).json({
        success: false,
        error: "Sender has no Arc wallet. CASHOUT first.",
      });
    }

    // Get or create receiver wallet
    let receiverWallet = userWallets.get(toPhone);
    if (!receiverWallet) {
      console.log(`   Creating Arc wallet for recipient ${toPhone}...`);
      const wallet = await walletService.createWallet();
      receiverWallet = {
        walletId: wallet.id,
        address: wallet.address,
        blockchain: wallet.blockchain,
      };
      userWallets.set(toPhone, receiverWallet);
      saveWallets();
    }

    // Get USDC token ID from sender's balances
    const balances = await walletService.getBalances(senderWallet.walletId);
    const usdcToken = balances.find(
      (b) =>
        b.token.symbol === "USDC" ||
        b.token.name.toLowerCase().includes("usd coin")
    );

    if (!usdcToken || parseFloat(usdcToken.amount) < parseFloat(amount)) {
      return res.status(400).json({
        success: false,
        error: `Insufficient USDC balance. Have: ${usdcToken?.amount ?? "0"}, Need: ${amount}`,
      });
    }

    // Transfer USDC
    const result = await walletService.transferUsdc(
      senderWallet.walletId,
      receiverWallet.address,
      amount,
      usdcToken.tokenId
    );

    console.log(
      `✅ USDC transfer: ${amount} from ${fromPhone} to ${toPhone}`
    );

    res.json({
      success: true,
      transactionId: result.transactionId,
      state: result.state,
      from: senderWallet.address,
      to: receiverWallet.address,
      amount,
    });
  } catch (error: any) {
    console.error("❌ Pay error:", error.message);
    res.status(500).json({ success: false, error: error.message });
  }
});

// ============================================================================
// GET /api/arc/wallets — List all registered wallets (admin/debug)
// ============================================================================
app.get("/api/arc/wallets", (_req, res) => {
  const wallets: any[] = [];
  userWallets.forEach((wallet, phone) => {
    wallets.push({ phone, ...wallet });
  });
  res.json({ success: true, count: wallets.length, wallets });
});

// ============================================================================
// Payout history (in-memory, persisted to file)
// ============================================================================
const PAYOUTS_FILE = path.join(__dirname, "..", "payouts.json");
interface PayoutRecord {
  id: string;
  type: "cashout" | "batch" | "pay";
  from: string;
  recipients: { phone: string; address: string; amount: string; status: string }[];
  totalAmount: string;
  token: string;
  chain: string;
  status: "pending" | "processing" | "complete" | "failed";
  txHashes: string[];
  createdAt: string;
  completedAt?: string;
}

let payoutHistory: PayoutRecord[] = [];

function loadPayouts() {
  try {
    if (fs.existsSync(PAYOUTS_FILE)) {
      payoutHistory = JSON.parse(fs.readFileSync(PAYOUTS_FILE, "utf8"));
      console.log(`📋 Loaded ${payoutHistory.length} payout records from disk`);
    }
  } catch (e: any) {
    console.error("⚠️  Failed to load payouts file:", e.message);
  }
}

function savePayouts() {
  try {
    fs.writeFileSync(PAYOUTS_FILE, JSON.stringify(payoutHistory, null, 2));
  } catch (e: any) {
    console.error("⚠️  Failed to save payouts file:", e.message);
  }
}

loadPayouts();

// ============================================================================
// POST /api/arc/batch-payout — Multi-recipient USDC payout on Arc
// Body: { fromPhone: string, recipients: [{ phone: string, amount: string }], memo?: string }
// ============================================================================
app.post("/api/arc/batch-payout", async (req, res) => {
  try {
    const { fromPhone, recipients, memo } = req.body;

    if (!fromPhone || !recipients || !Array.isArray(recipients) || recipients.length === 0) {
      return res.status(400).json({
        success: false,
        error: "Missing fromPhone or recipients array",
      });
    }

    const senderWallet = userWallets.get(fromPhone);
    if (!senderWallet) {
      return res.status(404).json({
        success: false,
        error: "Sender has no Arc wallet. CASHOUT first.",
      });
    }

    // Calculate total
    const totalAmount = recipients.reduce(
      (sum: number, r: any) => sum + parseFloat(r.amount || "0"),
      0
    );

    console.log(`\n📤 Batch Payout: ${totalAmount} USDC to ${recipients.length} recipients`);
    console.log(`   From: ${fromPhone} (${senderWallet.address})`);
    if (memo) console.log(`   Memo: ${memo}`);

    // Check sender balance
    const balances = await walletService.getBalances(senderWallet.walletId);
    const usdcToken = balances.find(
      (b) => b.token.symbol === "USDC" || b.token.name.toLowerCase().includes("usd coin")
    );

    if (!usdcToken || parseFloat(usdcToken.amount) < totalAmount) {
      return res.status(400).json({
        success: false,
        error: `Insufficient USDC. Have: ${usdcToken?.amount ?? "0"}, Need: ${totalAmount}`,
      });
    }

    // Create payout record
    const payoutId = `payout_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
    const record: PayoutRecord = {
      id: payoutId,
      type: "batch",
      from: fromPhone,
      recipients: [],
      totalAmount: totalAmount.toFixed(2),
      token: "USDC",
      chain: "ARC-TESTNET",
      status: "processing",
      txHashes: [],
      createdAt: new Date().toISOString(),
    };

    // Respond immediately
    res.json({
      success: true,
      payoutId,
      message: `Batch payout initiated: ${totalAmount} USDC to ${recipients.length} recipients`,
      recipientCount: recipients.length,
      totalAmount: totalAmount.toFixed(2),
    });

    // Process payouts asynchronously
    (async () => {
      for (const recipient of recipients) {
        try {
          // Get or create recipient wallet
          let recipientWallet = userWallets.get(recipient.phone);
          if (!recipientWallet) {
            console.log(`   Creating wallet for ${recipient.phone}...`);
            const wallet = await walletService.createWallet();
            recipientWallet = {
              walletId: wallet.id,
              address: wallet.address,
              blockchain: wallet.blockchain,
            };
            userWallets.set(recipient.phone, recipientWallet);
            saveWallets();
          }

          // Transfer USDC
          const result = await walletService.transferUsdc(
            senderWallet.walletId,
            recipientWallet.address,
            recipient.amount,
            usdcToken!.tokenId
          );

          record.recipients.push({
            phone: recipient.phone,
            address: recipientWallet.address,
            amount: recipient.amount,
            status: "complete",
          });
          record.txHashes.push(result.transactionId);

          console.log(`   ✅ ${recipient.amount} USDC → ${recipient.phone}`);
        } catch (err: any) {
          console.error(`   ❌ Failed for ${recipient.phone}: ${err.message}`);
          record.recipients.push({
            phone: recipient.phone,
            address: "",
            amount: recipient.amount,
            status: "failed",
          });
        }
      }

      record.status = record.recipients.every((r) => r.status === "complete")
        ? "complete"
        : "failed";
      record.completedAt = new Date().toISOString();
      payoutHistory.push(record);
      savePayouts();

      console.log(`\n✅ Batch payout ${payoutId} complete: ${record.recipients.filter((r) => r.status === "complete").length}/${recipients.length} succeeded`);
    })();
  } catch (error: any) {
    console.error("❌ Batch payout error:", error.message);
    res.status(500).json({ success: false, error: error.message });
  }
});

// ============================================================================
// GET /api/arc/treasury — Treasury dashboard data
// ============================================================================
app.get("/api/arc/treasury", async (_req, res) => {
  try {
    // Gather all wallet balances
    const walletData: any[] = [];
    let totalUsdc = 0;
    let totalWallets = 0;

    for (const [phone, wallet] of userWallets.entries()) {
      try {
        const usdcBalance = await walletService.getUsdcBalance(wallet.walletId);
        const bal = parseFloat(usdcBalance);
        totalUsdc += bal;
        totalWallets++;
        walletData.push({
          phone,
          address: wallet.address,
          usdcBalance: usdcBalance,
        });
      } catch (e) {
        walletData.push({ phone, address: wallet.address, usdcBalance: "0" });
      }
    }

    // Payout stats
    const totalPayouts = payoutHistory.length;
    const totalPaidOut = payoutHistory
      .filter((p) => p.status === "complete")
      .reduce((sum, p) => sum + parseFloat(p.totalAmount), 0);
    const recentPayouts = payoutHistory.slice(-10).reverse();

    res.json({
      success: true,
      treasury: {
        totalUsdc: totalUsdc.toFixed(2),
        totalWallets,
        chain: "ARC-TESTNET",
        token: "USDC",
      },
      payoutStats: {
        totalPayouts,
        totalPaidOut: totalPaidOut.toFixed(2),
        successRate: totalPayouts > 0
          ? ((payoutHistory.filter((p) => p.status === "complete").length / totalPayouts) * 100).toFixed(1) + "%"
          : "N/A",
      },
      wallets: walletData,
      recentPayouts,
    });
  } catch (error: any) {
    console.error("❌ Treasury error:", error.message);
    res.status(500).json({ success: false, error: error.message });
  }
});

// ============================================================================
// GET /api/arc/payouts — Payout history
// ============================================================================
app.get("/api/arc/payouts", (_req, res) => {
  res.json({
    success: true,
    count: payoutHistory.length,
    payouts: payoutHistory.slice().reverse(),
  });
});

// ============================================================================
// GET /api/arc/payout/:id — Single payout details
// ============================================================================
app.get("/api/arc/payout/:id", (req, res) => {
  const payout = payoutHistory.find((p) => p.id === req.params.id);
  if (!payout) {
    return res.status(404).json({ success: false, error: "Payout not found" });
  }
  res.json({ success: true, payout });
});

// ============================================================================
// Start server
// ============================================================================
const PORT = parseInt(process.env.PORT || "8084");

async function start() {
  try {
    // Initialize Circle Wallet Service
    console.log("🔄 Initializing Circle Wallet Service...");
    walletService = new CircleWalletService();
    await walletService.initialize();
    console.log("✅ Circle Wallet Service ready");

    // Initialize Cashout Service
    console.log("🔄 Initializing Cashout Service...");
    cashoutService = new CashoutService(walletService);
    console.log("✅ Cashout Service ready");

    app.listen(PORT, () => {
      console.log(`\n🚀 Arc Service running on port ${PORT}`);
      console.log(`   Health: http://localhost:${PORT}/health`);
      console.log(`   Wallet: POST http://localhost:${PORT}/api/arc/wallet`);
      console.log(`   Quote:  POST http://localhost:${PORT}/api/arc/quote`);
      console.log(
        `   Cashout: POST http://localhost:${PORT}/api/arc/cashout`
      );
      console.log(`   Pay:    POST http://localhost:${PORT}/api/arc/pay`);
      console.log(
        `   Balance: GET http://localhost:${PORT}/api/arc/balance/:phone`
      );
    });
  } catch (error: any) {
    console.error("❌ Failed to start Arc Service:", error.message);
    process.exit(1);
  }
}

start();
