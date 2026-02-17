/**
 * NEAR Contract Service for Text-to-Chain Backend
 * Handles all NEAR smart contract interactions
 */

import { Near, keyStores, Contract, Account, utils, KeyPair } from 'near-api-js';
import { NEAR_TESTNET_CONFIG } from './near.config';

export class NearContractService {
  private near!: Near;
  private contract!: Contract | any;
  private account!: Account;
  private initialized: boolean = false;
  private privateKey: string;

  constructor(privateKey: string) {
    this.privateKey = privateKey;
  }

  async initialize() {
    if (this.initialized) return;

    const ownerAccount = process.env.NEAR_OWNER_ACCOUNT || '0xswarnim.testnet';
    const poolContract = NEAR_TESTNET_CONFIG.contracts.poolContract;

    // Parse private key
    const keyPair = KeyPair.fromString(this.privateKey);
    const keyStore = new keyStores.InMemoryKeyStore();
    await keyStore.setKey(NEAR_TESTNET_CONFIG.networkId, ownerAccount, keyPair);

    // Create NEAR connection
    this.near = new Near({
      networkId: NEAR_TESTNET_CONFIG.networkId,
      nodeUrl: NEAR_TESTNET_CONFIG.nodeUrl,
      walletUrl: NEAR_TESTNET_CONFIG.walletUrl,
      keyStore,
    });

    // Get account
    this.account = await this.near.account(ownerAccount);

    // Initialize contract
    this.contract = new Contract(
      this.account,
      poolContract,
      {
        viewMethods: [
          'get_balance',
          'get_total_liquidity',
          'is_user_registered',
          'get_reserves',
          'get_pool_info',
        ],
        changeMethods: [
          'register_user',
          'deposit',
          'withdraw',
          'redeem_voucher',
          'swap_txtc_to_near',
          'swap_near_to_txtc',
        ],
        useLocalViewExecution: false,
      }
    );

    this.initialized = true;
    console.log(`✅ NEAR Service initialized: ${ownerAccount} → ${poolContract}`);
  }

  /**
   * Create NEAR account for user
   */
  async createAccount(accountId: string, publicKey: string): Promise<{
    success: boolean;
    txHash: string;
  }> {
    try {
      await this.initialize();
      console.log(`👤 Creating NEAR account: ${accountId}`);

      const result = await this.account.createAccount(
        accountId,
        publicKey,
        utils.format.parseNearAmount('0.1') || '0'
      );

      console.log(`✅ Account created: ${result.transaction.hash}`);

      return {
        success: true,
        txHash: result.transaction.hash,
      };
    } catch (error: any) {
      console.error('❌ Create account error:', error.message);
      // If account already exists, return success
      if (error.message?.includes('already exists')) {
        return { success: true, txHash: 'existing-account' };
      }
      throw new Error(`Failed to create account: ${error.message}`);
    }
  }

  /**
   * Get balance - supports phone number or NEAR account
   */
  async getBalance(identifier: string): Promise<{
    txtc: string;
    near: string;
    nearAccount?: string;
  }> {
    try {
      await this.initialize();

      // If it looks like a phone number, query by phone
      const isPhone = identifier.startsWith('+');

      if (isPhone) {
        const result = await this.contract.get_balance({ phone_number: identifier });
        return {
          txtc: utils.format.formatNearAmount(result.txtc_balance || '0') || '0',
          near: utils.format.formatNearAmount(result.near_balance || '0') || '0',
          nearAccount: result.near_account,
        };
      } else {
        // Query by NEAR account directly
        const account = await this.near.account(identifier);
        const nearBalance = await account.getAccountBalance();

        // Try to get TXTC balance from token contract
        const txtcContract = process.env.TXTC_TOKEN_CONTRACT || 'txtc.0xswarnim.testnet';
        let txtcBalance = '0';
        try {
          const contract = new Contract(account, txtcContract, {
            viewMethods: ['ft_balance_of'],
            changeMethods: [],
            useLocalViewExecution: false,
          }) as any;
          txtcBalance = await contract.ft_balance_of({ account_id: identifier });
        } catch (e) {
          // Token balance not available
        }

        return {
          txtc: utils.format.formatNearAmount(txtcBalance) || '0',
          near: utils.format.formatNearAmount(nearBalance.available) || '0',
          nearAccount: identifier,
        };
      }
    } catch (error: any) {
      console.error('❌ Get balance error:', error.message);
      throw new Error(`Failed to get balance: ${error.message}`);
    }
  }

  /**
   * Transfer tokens between users
   */
  async transfer(fromPhone: string, toPhone: string, amount: string): Promise<{
    success: boolean;
    txHash: string;
  }> {
    try {
      await this.initialize();
      console.log(`🔄 Transfer: ${amount} from ${fromPhone} to ${toPhone}`);

      const result = await (this.contract as any).transfer({
        args: {
          from_phone: fromPhone,
          to_phone: toPhone,
          amount: utils.format.parseNearAmount(amount) || '0',
        },
        gas: '300000000000000',
      });

      return {
        success: true,
        txHash: result.transaction?.hash || 'unknown',
      };
    } catch (error: any) {
      console.error('❌ Transfer error:', error.message);
      throw new Error(`Failed to transfer: ${error.message}`);
    }
  }

  /**
   * Redeem voucher code
   */
  async redeemVoucher(
    voucherCode: string,
    userAccount: string,
    nearAmount: string = '0.1',
    includeGas: boolean = true
  ): Promise<{
    success: boolean;
    txHash: string;
    tokenAmount: string;
    nearAmount: string;
  }> {
    try {
      await this.initialize();
      console.log(`🎟️  Redeeming voucher ${voucherCode} for ${userAccount}`);

      const result = await (this.contract as any).redeem_voucher({
        args: {
          code: voucherCode,
          recipient: userAccount,
        },
        gas: '300000000000000',
      });

      return {
        success: true,
        txHash: result.transaction?.hash || 'unknown',
        tokenAmount: '500', // Default voucher amount
        nearAmount: includeGas ? nearAmount : '0',
      };
    } catch (error: any) {
      console.error('❌ Redeem error:', error.message);
      throw new Error(`Failed to redeem voucher: ${error.message}`);
    }
  }

  /**
   * Swap TXTC to NEAR
   */
  async swapTokenForNear(userAddress: string, amount: string, minNearOut: string): Promise<{
    success: boolean;
    txHash: string;
    nearReceived: string;
  }> {
    try {
      await this.initialize();
      console.log(`🔄 Swap: ${amount} TXTC → NEAR for ${userAddress}`);

      const result = await (this.contract as any).swap_txtc_to_near({
        args: {
          user_account: userAddress,
          txtc_amount: utils.format.parseNearAmount(amount) || '0',
          min_near_out: minNearOut,
        },
        gas: '300000000000000',
      });

      return {
        success: true,
        txHash: result.transaction?.hash || 'unknown',
        nearReceived: '0.9', // Mock value
      };
    } catch (error: any) {
      console.error('❌ Swap error:', error.message);
      throw new Error(`Failed to swap: ${error.message}`);
    }
  }

  /**
   * Swap NEAR to TXTC
   */
  async swapNearForToken(userAddress: string, amount: string, minTokenOut: string): Promise<{
    success: boolean;
    txHash: string;
    tokenReceived: string;
  }> {
    try {
      await this.initialize();
      console.log(`🔄 Swap: ${amount} NEAR → TXTC for ${userAddress}`);

      const result = await (this.contract as any).swap_near_to_txtc({
        args: {
          user_account: userAddress,
          min_txtc_out: minTokenOut,
        },
        gas: '300000000000000',
        amount: utils.format.parseNearAmount(amount) || '0',
      });

      return {
        success: true,
        txHash: result.transaction?.hash || 'unknown',
        tokenReceived: '100', // Mock value
      };
    } catch (error: any) {
      console.error('❌ Swap error:', error.message);
      throw new Error(`Failed to swap: ${error.message}`);
    }
  }

  /**
   * Send TXTC tokens to another address
   */
  async sendTokens(fromAddress: string, toAddress: string, amount: string): Promise<{
    success: boolean;
    txHash: string;
  }> {
    try {
      await this.initialize();
      console.log(`📤 Send: ${amount} TXTC from ${fromAddress} to ${toAddress}`);

      const txtcContract = process.env.TXTC_TOKEN_CONTRACT || 'txtc.0xswarnim.testnet';

      const result = await this.account.functionCall({
        contractId: txtcContract,
        methodName: 'ft_transfer',
        args: {
          receiver_id: toAddress,
          amount: utils.format.parseNearAmount(amount) || '0',
        },
        gas: BigInt('30000000000000'),
        attachedDeposit: BigInt('1'),
      });

      return {
        success: true,
        txHash: result.transaction.hash,
      };
    } catch (error: any) {
      console.error('❌ Send tokens error:', error.message);
      throw new Error(`Failed to send tokens: ${error.message}`);
    }
  }

  /**
   * Mint TXTC tokens (for testing/admin)
   */
  async mintTxtc(userAddress: string, amount: string): Promise<string> {
    try {
      await this.initialize();
      console.log(`💎 Minting ${amount} TXTC for ${userAddress}`);

      const txtcContract = process.env.TXTC_TOKEN_CONTRACT || 'txtc.0xswarnim.testnet';

      const result = await this.account.functionCall({
        contractId: txtcContract,
        methodName: 'mint',
        args: {
          account_id: userAddress,
          amount: utils.format.parseNearAmount(amount) || '0',
        },
        gas: BigInt('30000000000000'),
      });

      return result.transaction.hash;
    } catch (error: any) {
      console.error('❌ Mint error:', error.message);
      throw new Error(`Failed to mint: ${error.message}`);
    }
  }

  /**
   * Get current TXTC/NEAR price
   */
  async getCurrentPrice(): Promise<string> {
    try {
      await this.initialize();
      const poolInfo = await (this.contract as any).get_pool_info();
      return '0.01'; // Mock price
    } catch (error: any) {
      console.error('❌ Get price error:', error.message);
      return '0.01';
    }
  }

  /**
   * Estimate swap output
   */
  async estimateSwapOutput(amount: string, isTokenToEth: boolean): Promise<string> {
    try {
      await this.initialize();
      // Simple estimation: 1 TXTC = 0.01 NEAR
      const amountNum = parseFloat(amount);
      if (isTokenToEth) {
        return (amountNum * 0.01).toFixed(4);
      } else {
        return (amountNum * 100).toFixed(4);
      }
    } catch (error: any) {
      console.error('❌ Estimate error:', error.message);
      return '0';
    }
  }

  /**
   * Get total pool liquidity
   */
  async getTotalLiquidity(): Promise<string> {
    try {
      await this.initialize();
      const liquidity = await (this.contract as any).get_total_liquidity();
      return utils.format.formatNearAmount(liquidity) || '0';
    } catch (error: any) {
      console.error('❌ Get liquidity error:', error.message);
      return '0';
    }
  }

  /**
   * Register user with phone number
   */
  async registerUser(phoneNumber: string, nearAccount: string): Promise<{
    success: boolean;
    txHash: string;
  }> {
    try {
      await this.initialize();
      console.log(`📝 Registering user ${phoneNumber} → ${nearAccount}`);

      const result = await (this.contract as any).register_user({
        args: {
          phone_number: phoneNumber,
          near_account: nearAccount,
        },
        gas: '300000000000000',
      });

      return {
        success: true,
        txHash: result.transaction?.hash || 'mock-tx-' + Date.now(),
      };
    } catch (error: any) {
      console.error('❌ Register error:', error.message);
      // Return success anyway for testing
      return {
        success: true,
        txHash: 'mock-register-' + Date.now(),
      };
    }
  }

  /**
   * Deposit NEAR to pool
   */
  async deposit(phoneNumber: string, amount: string): Promise<{
    success: boolean;
    txHash: string;
  }> {
    try {
      await this.initialize();
      console.log(`💰 Deposit: ${amount} NEAR for ${phoneNumber}`);

      const result = await (this.contract as any).deposit({
        args: { phone_number: phoneNumber },
        gas: '300000000000000',
        amount: utils.format.parseNearAmount(amount) || '0',
      });

      return {
        success: true,
        txHash: result.transaction?.hash || 'mock-deposit-' + Date.now(),
      };
    } catch (error: any) {
      console.error('❌ Deposit error:', error.message);
      throw new Error(`Failed to deposit: ${error.message}`);
    }
  }

  /**
   * Withdraw NEAR from pool
   */
  async withdraw(phoneNumber: string, amount: string): Promise<{
    success: boolean;
    txHash: string;
  }> {
    try {
      await this.initialize();
      console.log(`💸 Withdraw: ${amount} NEAR for ${phoneNumber}`);

      const result = await (this.contract as any).withdraw({
        args: {
          phone_number: phoneNumber,
          amount: utils.format.parseNearAmount(amount) || '0',
        },
        gas: '300000000000000',
      });

      return {
        success: true,
        txHash: result.transaction?.hash || 'mock-withdraw-' + Date.now(),
      };
    } catch (error: any) {
      console.error('❌ Withdraw error:', error.message);
      throw new Error(`Failed to withdraw: ${error.message}`);
    }
  }

  /**
   * Send gas fee to user
   */
  async sendGasFee(toAccount: string, amount: string): Promise<string> {
    try {
      await this.initialize();
      console.log(`⛽ Sending ${amount} NEAR gas to ${toAccount}`);

      const result = await this.account.sendMoney(
        toAccount,
        utils.format.parseNearAmount(amount) || '0'
      );

      return result.transaction.hash;
    } catch (error: any) {
      console.error('❌ Send gas error:', error.message);
      throw new Error(`Failed to send gas: ${error.message}`);
    }
  }
}

// Export singleton instance
let nearContractServiceInstance: NearContractService | null = null;

export function getNearContractService(privateKey?: string): NearContractService {
  if (!nearContractServiceInstance) {
    if (!privateKey) {
      throw new Error('Private key required for first initialization');
    }
    nearContractServiceInstance = new NearContractService(privateKey);
  }
  return nearContractServiceInstance;
}
