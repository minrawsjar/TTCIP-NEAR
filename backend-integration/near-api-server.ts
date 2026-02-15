/**
 * NEAR-based Contract API Server
 * Provides HTTP endpoints for SMS handler to call NEAR smart contracts
 */

import * as dotenv from "dotenv";
import "dotenv/config";
import express from "express";
import { NEAR_TESTNET_CONFIG } from "./near.config.ts";
import { getNearContractService } from "./near-contract-service.ts";
import { nearBlockchainMonitor } from "./near-blockchain-monitor.ts";
import { nearNameService } from "./near-name-service.ts";
import twilio from "twilio";

const app = express();
app.use(express.json());

// Initialize NEAR contract service
const nearContractService = getNearContractService(process.env.NEAR_PRIVATE_KEY!);

// Initialize Twilio
let twilioClient: any = null;
let twilioPhoneNumber: string = "";
const accountSid = process.env.TWILIO_ACCOUNT_SID;
const authToken = process.env.TWILIO_AUTH_TOKEN;
twilioPhoneNumber = process.env.TWILIO_PHONE_NUMBER || "";

if (accountSid && authToken) {
  twilioClient = twilio(accountSid, authToken);
  console.log("✅ Twilio SMS initialized");
} else {
  console.warn("⚠️  Twilio credentials not configured - SMS notifications disabled");
}

// Helper: look up user by NEAR account via SMS handler admin API
async function getUserByNearAccount(nearAccount: string): Promise<{ phone: string; ensName?: string } | null> {
  try {
    const smsHandlerUrl = process.env.SMS_HANDLER_URL || 'http://sms-handler:8080';
    const response = await fetch(`${smsHandlerUrl}/admin/wallets`, {
      headers: { 'Authorization': `Bearer ${process.env.ADMIN_TOKEN || 'admin123'}` },
    });
    const data = await response.json() as any;
    const users = data.wallets || [];
    const user = users.find((u: any) =>
      u.wallet_address.toLowerCase() === nearAccount.toLowerCase()
    );
    return user ? { phone: user.phone, ensName: user.ens_name } : null;
  } catch {
    return null;
  }
}

// Health check
app.get("/health", (req, res) => {
  res.json({
    status: "ok",
    network: "near-testnet",
    chainId: 441129640,
    contract: NEAR_TESTNET_CONFIG.contracts.poolContract
  });
});

// ============================================================================
// NEAR Pool Contract Endpoints
// ============================================================================

// Combined JOIN: create NEAR account + register in pool (SMS: JOIN alice)
app.post("/api/join", async (req, res) => {
  try {
    const { phoneNumber, name, userPhone } = req.body;

    if (!phoneNumber) {
      return res.status(400).json({ success: false, error: "Missing phoneNumber" });
    }

    const ownerPrefix = (process.env.NEAR_OWNER_ACCOUNT || "ttcip").replace(/\.(testnet|near)$/, "");
    const cleanName = (name || "")
      .toLowerCase()
      .trim()
      .replace(/[^a-z0-9_-]/g, "") || `user${phoneNumber.replace(/\D/g, "").slice(-6)}`;
    const nearAccount = `${cleanName}.${ownerPrefix}.testnet`;

    console.log(`📝 JOIN: ${phoneNumber} → ${nearAccount}`);

    // Check name availability
    const check = await nearNameService.checkAvailability(cleanName);
    if (!check.available) {
      return res.status(400).json({
        success: false,
        error: check.reason || "Name not available",
      });
    }

    // Create NEAR account and register in pool
    try {
      const { KeyPair } = await import("near-api-js");
      const keyPair = KeyPair.fromRandom("ed25519");
      const createResult = await nearContractService.createAccount(
        nearAccount,
        keyPair.getPublicKey().toString()
      );
      if (!createResult.success) throw new Error("Account creation failed");
    } catch (createErr: any) {
      if (!createErr.message?.includes("already exists") && !createErr.message?.includes("already exist")) {
        console.error("Account create error:", createErr.message);
        return res.status(500).json({
          success: false,
          error: `Account creation failed: ${createErr.message}`,
        });
      }
    }

    const regResult = await nearContractService.registerUser(phoneNumber, nearAccount);

    if (twilioClient && twilioPhoneNumber && userPhone) {
      try {
        const msg = `✅ NEAR wallet created!\n\nAccount: ${nearAccount}\n\nReply DEPOSIT for address, BALANCE to check.`;
        await twilioClient.messages.create({
          body: msg,
          from: twilioPhoneNumber,
          to: userPhone,
        });
      } catch (e: any) {
        console.error("SMS error:", e.message);
      }
    }

    res.json({
      success: true,
      nearAccount,
      txHash: regResult.txHash,
    });
  } catch (error: any) {
    console.error("❌ Join error:", error.message);
    res.status(500).json({ success: false, error: error.message });
  }
});

// Register user with phone number (existing NEAR account)
app.post("/api/register", async (req, res) => {
  try {
    const { phoneNumber, nearAccount, userPhone } = req.body;

    if (!phoneNumber || !nearAccount) {
      return res.status(400).json({
        success: false,
        error: "Missing phoneNumber or nearAccount",
      });
    }

    console.log(`📝 Registering NEAR user: ${phoneNumber} → ${nearAccount}`);

    const result = await nearContractService.registerUser(phoneNumber, nearAccount);

    // Send SMS notification
    if (twilioClient && twilioPhoneNumber && userPhone) {
      try {
        const message = `✅ NEAR wallet registered!\n\nPhone: ${phoneNumber}\nAccount: ${nearAccount}\n\nReply DEPOSIT to fund your wallet.`;

        await twilioClient.messages.create({
          body: message,
          from: twilioPhoneNumber,
          to: userPhone,
        });
        console.log(`📱 Registration SMS sent to ${userPhone}`);
      } catch (smsError: any) {
        console.error(`⚠️  Failed to send registration SMS: ${smsError.message}`);
      }
    }

    res.json({
      success: true,
      txHash: result.txHash,
      phoneNumber,
      nearAccount,
    });
  } catch (error: any) {
    console.error("❌ Register error:", error.message);

    res.status(500).json({
      success: false,
      error: error.message,
    });
  }
});

// Get user balance - supports phone or NEAR account (EVM parity: balance by address)
app.get("/api/balance/:identifier", async (req, res) => {
  try {
    const { identifier } = req.params;

    console.log(`📊 Getting balance for ${identifier}`);

    const balance = await nearContractService.getBalance(identifier);

    res.json({
      success: true,
      address: identifier,
      balances: {
        txtc: balance.txtc,
        near: balance.near,
      },
      nearAccount: balance.nearAccount,
      network: "near-testnet",
    });
  } catch (error: any) {
    console.error("❌ Balance error:", error.message);

    res.status(500).json({
      success: false,
      error: error.message,
    });
  }
});

// Deposit NEAR to pool
app.post("/api/deposit", async (req, res) => {
  try {
    const { phoneNumber, amount, userPhone } = req.body;

    if (!phoneNumber || !amount) {
      return res.status(400).json({
        success: false,
        error: "Missing phoneNumber or amount",
      });
    }

    console.log(`💰 NEAR deposit: ${amount} NEAR for ${phoneNumber}`);

    // Respond immediately to avoid Twilio timeout
    res.json({ success: true, message: "Deposit initiated" });

    // Process async
    (async () => {
      try {
        const result = await nearContractService.deposit(phoneNumber, amount);

        console.log(`✅ Deposit complete: ${result.txHash}`);

        // Send SMS notification if phone number provided
        if (twilioClient && twilioPhoneNumber && userPhone) {
          try {
            const message = `✅ Deposit complete!\n\n${amount} NEAR deposited to your pool account\n\nReply BALANCE to check.`;

            await twilioClient.messages.create({
              body: message,
              from: twilioPhoneNumber,
              to: userPhone,
            });
            console.log(`📱 Deposit notification sent to ${userPhone}`);
          } catch (smsError: any) {
            console.error(`⚠️  Failed to send deposit notification: ${smsError.message}`);
          }
        }
      } catch (error: any) {
        console.error("❌ Deposit error:", error.message);

        // Send error notification if phone number provided
        if (twilioClient && twilioPhoneNumber && userPhone) {
          try {
            await twilioClient.messages.create({
              body: "❌ Deposit failed. Please try again later.",
              from: twilioPhoneNumber,
              to: userPhone,
            });
          } catch (smsError: any) {
            console.error(`⚠️  Failed to send error notification: ${smsError.message}`);
          }
        }
      }
    })();
  } catch (error: any) {
    console.error("❌ Deposit initiation error:", error.message);

    res.status(500).json({
      success: false,
      error: error.message,
    });
  }
});

// Withdraw NEAR from pool
app.post("/api/withdraw", async (req, res) => {
  try {
    const { phoneNumber, amount, userPhone } = req.body;

    if (!phoneNumber || !amount) {
      return res.status(400).json({
        success: false,
        error: "Missing phoneNumber or amount",
      });
    }

    console.log(`💸 NEAR withdrawal: ${amount} NEAR for ${phoneNumber}`);

    // Respond immediately to avoid Twilio timeout
    res.json({ success: true, message: "Withdrawal initiated" });

    // Process async
    (async () => {
      try {
        const result = await nearContractService.withdraw(phoneNumber, amount);

        console.log(`✅ Withdrawal complete: ${result.txHash}`);

        // Send SMS notification if phone number provided
        if (twilioClient && twilioPhoneNumber && userPhone) {
          try {
            const message = `✅ Withdrawal complete!\n\n${amount} NEAR sent to your account\n\nReply BALANCE to check.`;

            await twilioClient.messages.create({
              body: message,
              from: twilioPhoneNumber,
              to: userPhone,
            });
            console.log(`📱 Withdrawal notification sent to ${userPhone}`);
          } catch (smsError: any) {
            console.error(`⚠️  Failed to send withdrawal notification: ${smsError.message}`);
          }
        }
      } catch (error: any) {
        console.error("❌ Withdrawal error:", error.message);

        // Send error notification if phone number provided
        if (twilioClient && twilioPhoneNumber && userPhone) {
          try {
            await twilioClient.messages.create({
              body: "❌ Withdrawal failed. Please try again later.",
              from: twilioPhoneNumber,
              to: userPhone,
            });
          } catch (smsError: any) {
            console.error(`⚠️  Failed to send error notification: ${smsError.message}`);
          }
        }
      }
    })();
  } catch (error: any) {
    console.error("❌ Withdrawal initiation error:", error.message);

    res.status(500).json({
      success: false,
      error: error.message,
    });
  }
});

// Transfer NEAR between users
app.post("/api/transfer", async (req, res) => {
  try {
    const { fromPhone, toPhone, amount, userPhone } = req.body;

    if (!fromPhone || !toPhone || !amount) {
      return res.status(400).json({
        success: false,
        error: "Missing fromPhone, toPhone, or amount",
      });
    }

    console.log(`🔄 NEAR transfer: ${amount} NEAR from ${fromPhone} to ${toPhone}`);

    // Respond immediately to avoid Twilio timeout
    res.json({ success: true, message: "Transfer initiated" });

    // Process async
    (async () => {
      try {
        const result = await nearContractService.transfer(fromPhone, toPhone, amount);

        console.log(`✅ Transfer complete: ${result.txHash}`);

        // Send SMS notification to sender
        if (twilioClient && twilioPhoneNumber && userPhone) {
          try {
            const message = `✅ Transfer complete!\n\n${amount} NEAR sent to ${toPhone}\n\nReply BALANCE to check.`;

            await twilioClient.messages.create({
              body: message,
              from: twilioPhoneNumber,
              to: userPhone,
            });
            console.log(`📱 Transfer notification sent to ${userPhone}`);
          } catch (smsError: any) {
            console.error(`⚠️  Failed to send transfer notification: ${smsError.message}`);
          }
        }

        // Send SMS notification to recipient
        if (twilioClient && twilioPhoneNumber) {
          try {
            const recipientUser = await getUserByNearAccount(toPhone);
            if (recipientUser && recipientUser.phone !== userPhone) {
              await twilioClient.messages.create({
                body: `✅ Received ${amount} NEAR from ${fromPhone}\n\nReply BALANCE to check.`,
                from: twilioPhoneNumber,
                to: recipientUser.phone,
              });
              console.log(`   📱 Recipient notified: ${recipientUser.phone}`);
            }
          } catch (smsError: any) {
            console.error(`⚠️  Recipient SMS error: ${smsError.message}`);
          }
        }
      } catch (error: any) {
        console.error("❌ Transfer error:", error.message);

        // Send error notification if phone number provided
        if (twilioClient && twilioPhoneNumber && userPhone) {
          try {
            await twilioClient.messages.create({
              body: "❌ Transfer failed. Please try again later.",
              from: twilioPhoneNumber,
              to: userPhone,
            });
          } catch (smsError: any) {
            console.error(`⚠️  Failed to send error notification: ${smsError.message}`);
          }
        }
      }
    })();
  } catch (error: any) {
    console.error("❌ Transfer initiation error:", error.message);

    res.status(500).json({
      success: false,
      error: error.message,
    });
  }
});

// Cashout (Withdraw alias for SMS consistency)
app.post("/api/cashout", async (req, res) => {
  try {
    const { phoneNumber, amount, userPhone } = req.body;

    if (!phoneNumber || !amount) {
      return res.status(400).json({ success: false, error: "Missing phoneNumber or amount" });
    }

    console.log(`💸 CASHOUT request: ${amount} TXTC/NEAR for ${phoneNumber}`);

    // Respond immediately
    res.json({ success: true, message: "Cashout initiated" });

    // Process async (Using withdraw for now as we don't have CCTP)
    (async () => {
      try {
        // In a real scenario, this would swap TXTC -> USDC -> CCTP
        // For NEAR MVP, we'll assume it's withdrawing NEAR or swapping TXTC then withdrawing
        // For simplicity, we'll map CASHOUT to withdrawing NEAR from the pool
        const result = await nearContractService.withdraw(phoneNumber, amount);

        console.log(`✅ Cashout complete: ${result.txHash}`);

        if (twilioClient && twilioPhoneNumber && userPhone) {
          await twilioClient.messages.create({
            body: `✅ Cashout successful!\n\nSent ${amount} NEAR to your bank (simulated).\n\nReply BALANCE to check.`,
            from: twilioPhoneNumber,
            to: userPhone,
          });
        }
      } catch (error: any) {
        console.error("❌ Cashout error:", error.message);
        if (twilioClient && twilioPhoneNumber && userPhone) {
          await twilioClient.messages.create({
            body: "❌ Cashout failed. Please try again later.",
            from: twilioPhoneNumber,
            to: userPhone,
          });
        }
      }
    })();
  } catch (error: any) {
    res.status(500).json({ success: false, error: error.message });
  }
});

// ============================================================================
// EVM Parity Endpoints (same API surface as api-server.ts)
// ============================================================================

// Redeem voucher - EVM: redeem TXTC + ETH gas bonus
app.post("/api/redeem", async (req, res) => {
  try {
    const { voucherCode, userAddress, userPhone } = req.body;
    const userAccount = userAddress;

    if (!voucherCode || !userAccount) {
      return res.status(400).json({
        success: false,
        error: "Missing voucherCode or userAddress (NEAR account)",
      });
    }

    console.log(`📝 Redeeming voucher ${voucherCode} for ${userAccount}`);

    const result = await nearContractService.redeemVoucher(
      voucherCode,
      userAccount,
      '10',
      true
    );

    if (twilioClient && twilioPhoneNumber && userPhone) {
      try {
        const message = `✅ Voucher redeemed!\n\nReceived:\n${result.tokenAmount} TXTC\n${result.nearAmount} NEAR (gas)\n\nReply BALANCE to check.`;
        await twilioClient.messages.create({
          body: message,
          from: twilioPhoneNumber,
          to: userPhone,
        });
      } catch (smsError: any) {
        console.error(`⚠️  Failed to send SMS: ${smsError.message}`);
      }
    }

    res.json({
      success: true,
      tokenAmount: result.tokenAmount,
      ethAmount: result.nearAmount,
      txHash: result.txHash,
    });
  } catch (error: any) {
    console.error("❌ Redeem error:", error.message);
    res.status(500).json({ success: false, error: error.message });
  }
});

// Swap - supports TXTC→NEAR and NEAR→TXTC (EVM: swap tokens for ETH)
app.post("/api/swap", async (req, res) => {
  try {
    const { userAddress, tokenAmount, nearAmount, direction = "txtc_to_near", minEthOut = "0", userPhone } = req.body;

    if (!userAddress) {
      return res.status(400).json({
        success: false,
        error: "Missing userAddress",
      });
    }

    const isTxtcToNear = direction === "near_to_txtc" ? false : true;
    const amount = isTxtcToNear ? tokenAmount : nearAmount;

    if (!amount) {
      return res.status(400).json({
        success: false,
        error: "Missing tokenAmount or nearAmount",
      });
    }

    res.json({ success: true, message: "Swap initiated" });

    (async () => {
      try {
        if (isTxtcToNear) {
          const result = await nearContractService.swapTokenForNear(userAddress, amount, minEthOut);
          if (twilioClient && twilioPhoneNumber && userPhone) {
            await twilioClient.messages.create({
              body: `✅ Swap complete!\n\n${amount} TXTC → ${result.nearReceived} NEAR\n\nReply BALANCE to check.`,
              from: twilioPhoneNumber,
              to: userPhone,
            });
          }
        } else {
          const result = await nearContractService.swapNearForToken(userAddress, amount, "0");
          if (twilioClient && twilioPhoneNumber && userPhone) {
            await twilioClient.messages.create({
              body: `✅ Swap complete!\n\n${amount} NEAR → ${result.tokenReceived} TXTC\n\nReply BALANCE to check.`,
              from: twilioPhoneNumber,
              to: userPhone,
            });
          }
        }
      } catch (err: any) {
        console.error("❌ Swap error:", err.message);
        if (twilioClient && twilioPhoneNumber && userPhone) {
          await twilioClient.messages.create({
            body: "❌ Swap failed. Please try again later.",
            from: twilioPhoneNumber,
            to: userPhone,
          });
        }
      }
    })();
  } catch (error: any) {
    res.status(500).json({ success: false, error: error.message });
  }
});

// Send TXTC - EVM: send tokens
app.post("/api/send", async (req, res) => {
  try {
    const { fromAddress, toAddress, amount } = req.body;

    if (!fromAddress || !toAddress || !amount) {
      return res.status(400).json({
        success: false,
        error: "Missing fromAddress, toAddress, or amount",
      });
    }

    const result = await nearContractService.sendTokens(fromAddress, toAddress, amount);

    res.json({
      success: true,
      txHash: result.txHash,
    });
  } catch (error: any) {
    console.error("❌ Send error:", error.message);
    res.status(500).json({ success: false, error: error.message });
  }
});

// Buy TXTC with airtime - EVM: BUY mints TXTC
app.post("/api/buy", async (req, res) => {
  try {
    const { userAddress, amount, userPhone } = req.body;

    if (!userAddress || !amount) {
      return res.status(400).json({
        success: false,
        error: "Missing userAddress or amount",
      });
    }

    res.json({ success: true, message: "Buy initiated" });

    (async () => {
      try {
        const eurToUsd = parseFloat(process.env.EUR_TO_USD_RATE || '1.08');
        const txtcRate = parseFloat(process.env.USD_TO_TXTC_RATE || '100');
        const totalTxtc = parseFloat(amount) * eurToUsd * txtcRate * 0.9;
        const txHash = await nearContractService.mintTxtc(userAddress, totalTxtc.toFixed(4));

        if (twilioClient && twilioPhoneNumber && userPhone) {
          await twilioClient.messages.create({
            body: `✅ Purchase complete!\n\n€${amount} airtime → ${totalTxtc.toFixed(2)} TXTC\n\nReply BALANCE to check.`,
            from: twilioPhoneNumber,
            to: userPhone,
          });
        }
      } catch (err: any) {
        if (twilioClient && twilioPhoneNumber && userPhone) {
          await twilioClient.messages.create({
            body: `❌ Purchase failed: ${err.message}\n\nTry again later.`,
            from: twilioPhoneNumber,
            to: userPhone,
          });
        }
      }
    })();
  } catch (error: any) {
    res.status(500).json({ success: false, error: error.message });
  }
});

// Get price and quote - EVM parity
app.get("/api/price", async (req, res) => {
  try {
    const price = await nearContractService.getCurrentPrice();
    res.json({
      success: true,
      price,
      description: "1 TXTC = " + price + " NEAR",
    });
  } catch (error: any) {
    res.status(500).json({ success: false, error: (error as Error).message });
  }
});

app.post("/api/quote", async (req, res) => {
  try {
    const { amount, isTokenToEth = true } = req.body;
    const quote = await nearContractService.estimateSwapOutput(amount, isTokenToEth);
    res.json({
      success: true,
      inputAmount: amount,
      outputAmount: quote,
      direction: isTokenToEth ? "TXTC → NEAR" : "NEAR → TXTC",
    });
  } catch (error: any) {
    res.status(500).json({ success: false, error: (error as Error).message });
  }
});

// ============================================================================
// NEAR Name Service - ENS parity (*.ttcip.testnet)
// ============================================================================

app.get('/api/ens/check/:ensName', async (req, res) => {
  try {
    const { ensName } = req.params;
    const result = await nearNameService.checkAvailability(ensName);
    res.json({
      success: true,
      available: result.available,
      ensName: result.available ? `${ensName.toLowerCase()}.ttcip.testnet` : undefined,
      reason: result.reason,
    });
  } catch (error: any) {
    res.status(500).json({ success: false, error: error.message });
  }
});

app.post('/api/ens/register', async (req, res) => {
  try {
    const { ensName, walletAddress } = req.body;
    if (!ensName || !walletAddress) {
      return res.status(400).json({ success: false, error: 'Missing ensName or walletAddress' });
    }
    const result = await nearNameService.registerSubdomain(ensName, walletAddress);
    if (result.success) {
      res.json({
        success: true,
        ensName: result.ensName,
        walletAddress: result.nearAccount || walletAddress,
        txHash: result.txHash,
        message: `NEAR account ${result.ensName} registered`,
      });
    } else {
      res.status(400).json({ success: false, error: result.error });
    }
  } catch (error: any) {
    res.status(500).json({ success: false, error: (error as Error).message });
  }
});

app.get('/api/ens/resolve/:ensName', async (req, res) => {
  try {
    const { ensName } = req.params;
    const address = await nearNameService.resolveAddress(ensName);
    if (address) {
      res.json({ success: true, ensName, address });
    } else {
      res.status(404).json({ success: false, error: 'Name not found' });
    }
  } catch (error: any) {
    res.status(500).json({ success: false, error: (error as Error).message });
  }
});

// Arc/cashout notify - EVM parity (SMS notification hook)
app.post("/api/arc/notify", async (req, res) => {
  try {
    const { phone, message } = req.body;
    if (!phone || !message) {
      return res.status(400).json({ success: false, error: "Missing phone or message" });
    }
    if (!twilioClient || !twilioPhoneNumber) {
      return res.status(503).json({ success: false, error: "SMS service not configured" });
    }
    const smsResult = await twilioClient.messages.create({
      body: message,
      from: twilioPhoneNumber,
      to: phone,
    });
    res.json({ success: true, messageSid: smsResult.sid });
  } catch (error: any) {
    res.status(500).json({ success: false, error: error.message });
  }
});

// Get total pool liquidity
app.get("/api/pool/liquidity", async (req, res) => {
  try {
    const liquidity = await nearContractService.getTotalLiquidity();

    res.json({
      success: true,
      liquidity,
      contract: NEAR_TESTNET_CONFIG.contracts.poolContract,
      network: "near-testnet",
    });
  } catch (error: any) {
    console.error("❌ Liquidity error:", error.message);

    res.status(500).json({
      success: false,
      error: error.message,
    });
  }
});

// Contract addresses info
app.get("/api/contracts", (req, res) => {
  res.json({
    success: true,
    network: "near-testnet",
    chainId: 1313161555,
    contracts: NEAR_TESTNET_CONFIG.contracts,
    explorer: 'https://explorer.testnet.near.org',
  });
});

// Error handler
app.use(
  (
    err: any,
    req: express.Request,
    res: express.Response,
    next: express.NextFunction,
  ) => {
    console.error("Server error:", err);
    res.status(500).json({
      success: false,
      error: "Internal server error",
    });
  },
);

// Start server
const PORT = process.env.PORT || 3000;
app.listen(PORT, () => {
  console.log("🚀 NEAR Contract API Server Started");
  console.log("================================");
  console.log(`Port: ${PORT}`);
  console.log(`Network: ${NEAR_TESTNET_CONFIG.networkId}`);
  console.log(`Contract: ${NEAR_TESTNET_CONFIG.contracts.poolContract}`);
  console.log("\n📋 Available Endpoints:");
  console.log("  POST /api/register    - Register user");
  console.log("  GET  /api/balance/:phoneNumber - Get balance");
  console.log("  POST /api/deposit    - Deposit NEAR");
  console.log("  POST /api/withdraw   - Withdraw NEAR");
  console.log("  POST /api/transfer    - Transfer NEAR");
  console.log("  GET  /api/pool/liquidity - Get pool liquidity");
  console.log("  GET  /api/contracts - Contract addresses");
  console.log("  GET  /health        - Health check");
  console.log("\n✅ Ready to receive requests from SMS handler!");
  console.log("================================\n");

  setTimeout(() => {
    nearBlockchainMonitor.start().catch((err: any) => {
      console.error('❌ Failed to start NEAR blockchain monitor:', err.message);
    });
  }, 3000);
});

export default app;
