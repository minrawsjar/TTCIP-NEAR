import { ethers } from "ethers";
import { CircleWalletService } from "./circle-wallet";

// ============================================================================
// Cashout Service: TXTC → WETH → USDC (Uniswap V3) → Bridge CCTP → Arc
// ============================================================================

// Sepolia contract addresses
const TXTC_ADDRESS =
  process.env.TXTC_ADDRESS || "0x0F0E4A3F59C3B8794A9044a0dC0155fB3C3fA223";
const WETH_ADDRESS =
  process.env.WETH_ADDRESS || "0xfFf9976782d46CC05630D1f6eBAb18b2324d6B14";
const UNISWAP_V3_ROUTER =
  process.env.UNISWAP_V3_ROUTER ||
  "0x3bFA4769FB09eefC5a80d6E87c3B9C650f7Ae48E";
const POOL_FEE = parseInt(process.env.POOL_FEE || "3000");
const SEPOLIA_USDC = "0x1c7d4b196cb0c7b01d743fbc6116a902379c7238";
const WETH_USDC_POOL_FEE = 500; // Best liquidity WETH/USDC pool on Sepolia

// Uniswap V3 SwapRouter ABI (exactInputSingle)
const SWAP_ROUTER_ABI = [
  "function exactInputSingle((address tokenIn, address tokenOut, uint24 fee, address recipient, uint256 amountIn, uint256 amountOutMinimum, uint160 sqrtPriceLimitX96)) external payable returns (uint256 amountOut)",
];

// ERC20 ABI
const ERC20_ABI = [
  "function approve(address spender, uint256 amount) returns (bool)",
  "function balanceOf(address account) view returns (uint256)",
  "function allowance(address owner, address spender) view returns (uint256)",
  "function decimals() view returns (uint8)",
  "function symbol() view returns (string)",
];

// WETH ABI (for unwrapping)
const WETH_ABI = [
  ...ERC20_ABI,
  "function deposit() payable",
  "function withdraw(uint256 amount)",
];

// Uniswap V3 Pool ABI (for price quote)
const POOL_ABI = [
  "function slot0() view returns (uint160 sqrtPriceX96, int24 tick, uint16 observationIndex, uint16 observationCardinality, uint16 observationCardinalityNext, uint8 feeProtocol, bool unlocked)",
];

export interface CashoutResult {
  success: boolean;
  txtcAmount: string;
  wethReceived: string;
  usdcEstimate: string;
  swapTxHash: string;
  bridgeStatus: string;
  arcWalletAddress: string;
  error?: string;
}

export interface SwapQuote {
  txtcAmount: string;
  estimatedWeth: string;
  estimatedUsdc: string;
  rate: string;
}

export class CashoutService {
  private provider: ethers.JsonRpcProvider;
  private signer: ethers.Wallet;
  private walletService: CircleWalletService;

  constructor(walletService: CircleWalletService) {
    const rpcUrl =
      process.env.SEPOLIA_RPC_URL ||
      "https://eth-sepolia.g.alchemy.com/v2/pdI2-zhVUTWUFR0KVshcS";
    const privateKey = process.env.PRIVATE_KEY;

    if (!privateKey) {
      throw new Error("PRIVATE_KEY must be set in environment");
    }

    this.provider = new ethers.JsonRpcProvider(rpcUrl);
    this.signer = new ethers.Wallet(privateKey, this.provider);
    this.walletService = walletService;
  }

  // ============================================================================
  // Get swap quote: how much WETH/USDC for X TXTC
  // ============================================================================
  async getQuote(txtcAmount: string): Promise<SwapQuote> {
    const poolAddress =
      process.env.UNISWAP_V3_POOL ||
      "0xfdbf742dfc37b7ed1da429d3d7add78d99026c23";
    const pool = new ethers.Contract(poolAddress, POOL_ABI, this.provider);

    const slot0 = await pool.slot0();
    const sqrtPriceX96 = BigInt(slot0[0].toString());

    // Calculate price from sqrtPriceX96
    // price = (sqrtPriceX96 / 2^96)^2
    const Q96 = BigInt(2) ** BigInt(96);
    const priceNum = sqrtPriceX96 * sqrtPriceX96;
    const priceDen = Q96 * Q96;

    // TXTC is token0, WETH is token1
    // price = token1/token0 = WETH per TXTC
    const txtcWei = BigInt(ethers.parseEther(txtcAmount).toString());
    const wethOut = (txtcWei * priceNum) / priceDen;

    const wethFormatted = ethers.formatEther(wethOut);
    // Rough USDC estimate: 1 ETH ≈ $2500 (configurable)
    const ethPrice = parseFloat(process.env.ETH_PRICE_USD || "2500");
    const wethFloat = parseFloat(wethFormatted);
    const txtcFloat = parseFloat(txtcAmount);
    const usdcEstimate = (wethFloat * ethPrice).toFixed(2);
    const ratePerTxtc = txtcFloat > 0 ? (wethFloat / txtcFloat).toFixed(6) : "0";

    return {
      txtcAmount,
      estimatedWeth: wethFormatted,
      estimatedUsdc: usdcEstimate,
      rate: `1 TXTC ≈ ${ratePerTxtc} ETH`,
    };
  }

  // ============================================================================
  // Step 1: Swap TXTC → WETH on Uniswap V3 (Sepolia)
  // ============================================================================
  async swapTxtcToWeth(
    txtcAmount: string,
    userAddress: string
  ): Promise<{ wethReceived: string; txHash: string }> {
    console.log(`\n🔄 Swapping ${txtcAmount} TXTC → WETH on Sepolia...`);

    const txtcContract = new ethers.Contract(
      TXTC_ADDRESS,
      [...ERC20_ABI, "function mint(address to, uint256 amount)"],
      this.signer
    );
    const swapRouter = new ethers.Contract(
      UNISWAP_V3_ROUTER,
      SWAP_ROUTER_ABI,
      this.signer
    );

    const amountIn = ethers.parseEther(txtcAmount);

    // Mint TXTC to backend wallet for swap (user balance tracked off-chain)
    console.log(`   Minting ${txtcAmount} TXTC to backend for swap...`);
    const mintTx = await txtcContract.mint(await this.signer.getAddress(), amountIn);
    await mintTx.wait();
    console.log("   ✅ TXTC minted to backend");

    // Approve router to spend TXTC (backend wallet's TXTC for the swap)
    const currentAllowance = await txtcContract.allowance(
      await this.signer.getAddress(),
      UNISWAP_V3_ROUTER
    );
    if (currentAllowance < amountIn) {
      console.log("   Approving TXTC for swap router...");
      const approveTx = await txtcContract.approve(
        UNISWAP_V3_ROUTER,
        ethers.MaxUint256
      );
      await approveTx.wait();
      console.log("   ✅ Approved");
    }

    // Execute swap: TXTC → WETH
    const swapParams = {
      tokenIn: TXTC_ADDRESS,
      tokenOut: WETH_ADDRESS,
      fee: POOL_FEE,
      recipient: await this.signer.getAddress(),
      amountIn: amountIn,
      amountOutMinimum: 0n, // Accept any amount for testnet
      sqrtPriceLimitX96: 0n,
    };

    console.log("   Executing swap...");
    const tx = await swapRouter.exactInputSingle(swapParams);
    const receipt = await tx.wait();

    // Get WETH balance change (simplified - check balance after)
    const wethContract = new ethers.Contract(
      WETH_ADDRESS,
      ERC20_ABI,
      this.provider
    );
    const wethBalance = await wethContract.balanceOf(
      await this.signer.getAddress()
    );

    console.log(
      `   ✅ Swap complete! TX: ${receipt.hash}`
    );
    console.log(
      `   WETH balance: ${ethers.formatEther(wethBalance)}`
    );

    return {
      wethReceived: ethers.formatEther(wethBalance),
      txHash: receipt.hash,
    };
  }

  // ============================================================================
  // Step 2: Swap WETH → USDC on Sepolia (Uniswap V3 WETH/USDC pool)
  // ============================================================================
  async swapWethToUsdc(
    wethAmount: string
  ): Promise<{ usdcReceived: string; txHash: string }> {
    console.log(`\n💱 Swapping ${wethAmount} WETH → USDC on Sepolia...`);

    const wethContract = new ethers.Contract(WETH_ADDRESS, ERC20_ABI, this.signer);
    const swapRouter = new ethers.Contract(UNISWAP_V3_ROUTER, SWAP_ROUTER_ABI, this.signer);
    const backendAddress = await this.signer.getAddress();

    const amountIn = ethers.parseEther(wethAmount);

    // Approve router to spend WETH
    const currentAllowance = await wethContract.allowance(backendAddress, UNISWAP_V3_ROUTER);
    if (currentAllowance < amountIn) {
      console.log("   Approving WETH for swap router...");
      const approveTx = await wethContract.approve(UNISWAP_V3_ROUTER, ethers.MaxUint256);
      await approveTx.wait();
      console.log("   ✅ Approved");
    }

    // Execute swap: WETH → USDC (fee tier 500 = 0.05%)
    const swapParams = {
      tokenIn: WETH_ADDRESS,
      tokenOut: SEPOLIA_USDC,
      fee: WETH_USDC_POOL_FEE,
      recipient: backendAddress,
      amountIn: amountIn,
      amountOutMinimum: 0n, // Accept any amount for testnet
      sqrtPriceLimitX96: 0n,
    };

    console.log("   Executing WETH → USDC swap...");
    const tx = await swapRouter.exactInputSingle(swapParams);
    const receipt = await tx.wait();

    // Check USDC balance after swap
    const usdcContract = new ethers.Contract(SEPOLIA_USDC, ERC20_ABI, this.provider);
    const usdcBalance = await usdcContract.balanceOf(backendAddress);
    const usdcReceived = ethers.formatUnits(usdcBalance, 6);

    console.log(`   ✅ Swap complete! TX: ${receipt.hash}`);
    console.log(`   USDC received: ${usdcReceived}`);

    return {
      usdcReceived,
      txHash: receipt.hash,
    };
  }

  // ============================================================================
  // Step 3: Bridge USDC from Sepolia → Arc Testnet via CCTP
  // Real implementation using Circle's Cross-Chain Transfer Protocol:
  //   1. Approve USDC for TokenMessengerV2
  //   2. Burn USDC on Sepolia (depositForBurn)
  //   3. Poll Circle Iris API for attestation
  //   4. Mint USDC on Arc (receiveMessage)
  // ============================================================================

  // CCTP Contract Addresses
  private static SEPOLIA_TOKEN_MESSENGER = "0x8fe6b999dc680ccfdd5bf7eb0974218be2542daa";
  private static ARC_MESSAGE_TRANSMITTER = "0xe737e5cebeeba77efe34d4aa090756590b1ce275";
  private static SEPOLIA_DOMAIN = 0;
  private static ARC_DOMAIN = 26;
  private static IRIS_API = "https://iris-api-sandbox.circle.com";
  private static ARC_RPC = process.env.ARC_RPC_URL || "https://arc-testnet.drpc.org";

  // TokenMessengerV2 ABI
  private static TOKEN_MESSENGER_ABI = [
    "function depositForBurn(uint256 amount, uint32 destinationDomain, bytes32 mintRecipient, address burnToken, bytes32 destinationCaller, uint256 maxFee, uint32 minFinalityThreshold) external returns (uint64 nonce)",
  ];

  // MessageTransmitterV2 ABI
  private static MESSAGE_TRANSMITTER_ABI = [
    "function receiveMessage(bytes message, bytes attestation) external returns (bool success)",
  ];

  async bridgeToArc(
    usdcAmount: string,
    arcWalletAddress: string
  ): Promise<{ bridgeId: string; status: string; estimatedUsdc: string }> {
    console.log(
      `\n🌉 Bridging ${usdcAmount} USDC → Arc Testnet via CCTP...`
    );
    console.log(`   Destination: ${arcWalletAddress}`);

    const backendAddress = await this.signer.getAddress();

    // Use the actual USDC from the WETH→USDC swap (already in wallet)
    const usdcContract = new ethers.Contract(SEPOLIA_USDC, ERC20_ABI, this.signer);
    const usdcBalance = await usdcContract.balanceOf(backendAddress);
    console.log(`   USDC balance (from swap): ${ethers.formatUnits(usdcBalance, 6)} USDC`);

    if (usdcBalance === 0n) {
      return {
        bridgeId: "no_usdc",
        status: "FAILED_NO_USDC",
        estimatedUsdc: usdcAmount,
      };
    }

    // Bridge all USDC from the swap
    const bridgeAmount = usdcBalance;
    const actualUsdc = ethers.formatUnits(bridgeAmount, 6);
    console.log(`   Bridging ${actualUsdc} USDC via CCTP...`);

    // Step 3a: Approve USDC for TokenMessengerV2
    console.log("   📝 Approving USDC for TokenMessengerV2...");
    const allowance = await usdcContract.allowance(
      backendAddress,
      CashoutService.SEPOLIA_TOKEN_MESSENGER
    );
    if (allowance < bridgeAmount) {
      const approveTx = await usdcContract.approve(
        CashoutService.SEPOLIA_TOKEN_MESSENGER,
        ethers.MaxUint256
      );
      await approveTx.wait();
      console.log("   ✅ USDC approved");
    }

    // Step 3b: Burn USDC on Sepolia via depositForBurn
    console.log("   🔥 Burning USDC on Sepolia...");
    const tokenMessenger = new ethers.Contract(
      CashoutService.SEPOLIA_TOKEN_MESSENGER,
      CashoutService.TOKEN_MESSENGER_ABI,
      this.signer
    );

    // Format destination address as bytes32
    const mintRecipient = "0x000000000000000000000000" + arcWalletAddress.slice(2);
    const destinationCaller = "0x" + "0".repeat(64); // Allow any caller

    // Dynamic maxFee: must be less than bridgeAmount
    // Use 50% of amount as maxFee cap, or fall back to standard finality if amount is tiny
    const maxFee = bridgeAmount / 2n;
    const minFinalityThreshold = maxFee > 0n ? 1000 : 2000; // Fast Transfer if fee allows
    console.log(`   Using ${minFinalityThreshold === 1000 ? 'Fast' : 'Standard'} Transfer (maxFee: ${ethers.formatUnits(maxFee, 6)} USDC)`);

    const burnTx = await tokenMessenger.depositForBurn(
      bridgeAmount,
      CashoutService.ARC_DOMAIN,
      mintRecipient,
      SEPOLIA_USDC,
      destinationCaller,
      maxFee,
      minFinalityThreshold
    );
    const burnReceipt = await burnTx.wait();
    console.log(`   ✅ Burn TX: ${burnReceipt.hash}`);

    // Step 3c: Poll for attestation
    console.log("   ⏳ Waiting for CCTP attestation...");
    const attestation = await this.pollAttestation(burnReceipt.hash);

    if (attestation) {
      // Step 3d: Mint USDC on Arc
      console.log("   🪙 Minting USDC on Arc Testnet...");
      try {
        const arcProvider = new ethers.JsonRpcProvider(CashoutService.ARC_RPC);
        const arcSigner = new ethers.Wallet(process.env.PRIVATE_KEY!, arcProvider);
        const messageTransmitter = new ethers.Contract(
          CashoutService.ARC_MESSAGE_TRANSMITTER,
          CashoutService.MESSAGE_TRANSMITTER_ABI,
          arcSigner
        );

        const mintTx = await messageTransmitter.receiveMessage(
          attestation.message,
          attestation.attestation
        );
        const mintReceipt = await mintTx.wait();
        console.log(`   ✅ Mint TX on Arc: ${mintReceipt.hash}`);

        return {
          bridgeId: burnReceipt.hash,
          status: "COMPLETE",
          estimatedUsdc: actualUsdc,
        };
      } catch (mintError: any) {
        console.error(`   ❌ Mint failed: ${mintError.message}`);
        return {
          bridgeId: burnReceipt.hash,
          status: "ATTESTATION_READY_MINT_FAILED",
          estimatedUsdc: actualUsdc,
        };
      }
    } else {
      console.log("   ⏳ Attestation not ready yet — will complete async");
      return {
        bridgeId: burnReceipt.hash,
        status: "ATTESTATION_PENDING",
        estimatedUsdc: actualUsdc,
      };
    }
  }

  // ============================================================================
  // Poll Circle Iris API for CCTP attestation
  // ============================================================================
  private async pollAttestation(
    burnTxHash: string,
    maxAttempts: number = 60,
    intervalMs: number = 5000
  ): Promise<{ message: string; attestation: string } | null> {
    const url = `${CashoutService.IRIS_API}/v2/messages/${CashoutService.SEPOLIA_DOMAIN}?transactionHash=${burnTxHash}`;

    for (let i = 0; i < maxAttempts; i++) {
      try {
        const response = await fetch(url);
        if (response.ok) {
          const data: any = await response.json();
          if (data?.messages?.[0]?.status === "complete") {
            console.log(`   ✅ Attestation received (attempt ${i + 1})`);
            return {
              message: data.messages[0].message,
              attestation: data.messages[0].attestation,
            };
          }
        }
      } catch (error: any) {
        // Ignore fetch errors, keep polling
      }

      if (i < maxAttempts - 1) {
        console.log(`   ⏳ Polling attestation... (${i + 1}/${maxAttempts})`);
        await new Promise((resolve) => setTimeout(resolve, intervalMs));
      }
    }

    console.log("   ⚠️  Attestation timeout — will need manual completion");
    return null;
  }

  // ============================================================================
  // ETH Cashout: Wrap ETH → WETH → USDC (Sepolia) → Bridge CCTP → Arc
  // ============================================================================
  async cashoutEth(
    ethAmount: string,
    userAddress: string,
    arcWalletId: string,
    arcWalletAddress: string
  ): Promise<CashoutResult> {
    console.log("\n" + "=".repeat(60));
    console.log(`💰 CASHOUT ETH: ${ethAmount} ETH → USDC on Arc`);
    console.log(`   User: ${userAddress}`);
    console.log(`   Arc Wallet: ${arcWalletAddress}`);
    console.log("=".repeat(60));

    try {
      // Step 1: Wrap ETH → WETH
      console.log(`\n🔄 Wrapping ${ethAmount} ETH → WETH...`);
      const wethContract = new ethers.Contract(WETH_ADDRESS, WETH_ABI, this.signer);
      const amountWei = ethers.parseEther(ethAmount);
      const wrapTx = await wethContract.deposit({ value: amountWei });
      await wrapTx.wait();
      console.log(`   ✅ Wrapped ${ethAmount} ETH → WETH`);

      // Step 2: Swap WETH → USDC on Sepolia
      const usdcSwapResult = await this.swapWethToUsdc(ethAmount);

      // Step 3: Bridge USDC Sepolia → Arc via CCTP
      const bridgeResult = await this.bridgeToArc(
        usdcSwapResult.usdcReceived,
        arcWalletAddress
      );

      console.log("\n✅ ETH CASHOUT COMPLETE!");
      console.log(`   ${ethAmount} ETH → ${usdcSwapResult.usdcReceived} USDC → Arc`);

      return {
        success: true,
        txtcAmount: ethAmount,
        wethReceived: ethAmount,
        usdcEstimate: bridgeResult.estimatedUsdc,
        swapTxHash: wrapTx.hash,
        bridgeStatus: bridgeResult.status,
        arcWalletAddress,
      };
    } catch (error: any) {
      console.error("❌ ETH Cashout failed:", error.message);
      return {
        success: false,
        txtcAmount: ethAmount,
        wethReceived: "0",
        usdcEstimate: "0",
        swapTxHash: "",
        bridgeStatus: "FAILED",
        arcWalletAddress,
        error: error.message,
      };
    }
  }

  // ============================================================================
  // Full Cashout Flow: TXTC → WETH → USDC (Sepolia) → Bridge CCTP → Arc
  // ============================================================================
  async cashout(
    txtcAmount: string,
    userAddress: string,
    arcWalletId: string,
    arcWalletAddress: string
  ): Promise<CashoutResult> {
    console.log("\n" + "=".repeat(60));
    console.log(`💰 CASHOUT: ${txtcAmount} TXTC → USDC on Arc`);
    console.log(`   User: ${userAddress}`);
    console.log(`   Arc Wallet: ${arcWalletAddress}`);
    console.log("=".repeat(60));

    try {
      // Step 1: Swap TXTC → WETH on Sepolia
      const swapResult = await this.swapTxtcToWeth(txtcAmount, userAddress);

      // Step 2: Swap WETH → USDC on Sepolia
      const usdcSwapResult = await this.swapWethToUsdc(swapResult.wethReceived);

      // Step 3: Bridge USDC Sepolia → Arc via CCTP
      const bridgeResult = await this.bridgeToArc(
        usdcSwapResult.usdcReceived,
        arcWalletAddress
      );

      console.log("\n✅ CASHOUT COMPLETE!");
      console.log(`   ${txtcAmount} TXTC → ${swapResult.wethReceived} WETH → ${usdcSwapResult.usdcReceived} USDC → Arc`);

      return {
        success: true,
        txtcAmount,
        wethReceived: swapResult.wethReceived,
        usdcEstimate: bridgeResult.estimatedUsdc,
        swapTxHash: swapResult.txHash,
        bridgeStatus: bridgeResult.status,
        arcWalletAddress,
      };
    } catch (error: any) {
      console.error("❌ Cashout failed:", error.message);
      return {
        success: false,
        txtcAmount,
        wethReceived: "0",
        usdcEstimate: "0",
        swapTxHash: "",
        bridgeStatus: "FAILED",
        arcWalletAddress,
        error: error.message,
      };
    }
  }
}
