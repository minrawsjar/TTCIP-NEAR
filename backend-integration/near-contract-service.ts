/**
 * NEAR Contract Service for Text-to-Chain Backend
 * Handles all NEAR smart contract interactions
 */

import { Near, keyStores, Contract, Account, utils } from 'near-api-js';

export interface NearConfig {
  networkId: string;
  nodeUrl: string;
  walletUrl: string;
  helperUrl: string;
  contractName: string;
}

export const NEAR_TESTNET_CONFIG: NearConfig = {
  networkId: 'testnet',
  nodeUrl: 'https://rpc.testnet.near.org',
  walletUrl: 'https://testnet.mynearwallet.com/',
  helperUrl: 'https://helper.testnet.near.org',
  contractName: 'ttcip-pool.0xswarn.testnet',
};

export class NearContractService {
  private near: Near;
  private contract: Contract;
  private account: Account;
  
  constructor(privateKey: string, config: NearConfig = NEAR_TESTNET_CONFIG) {
    // Create NEAR connection from private key
    this.near = new Near({
      networkId: config.networkId,
      nodeUrl: config.nodeUrl,
      walletUrl: config.walletUrl,
      helperUrl: config.helperUrl,
      keyStore: new keyStores.InMemoryKeyStore(),
    });
  }

  private async initialize() {
    const config = NEAR_TESTNET_CONFIG;
    // Create account from private key
    this.account = await this.near.account(config.contractName);
    
    // Initialize contract
    this.contract = new Contract(
      this.account,
      config.contractName,
      {
        viewMethods: [
          'get_balance',
          'get_total_liquidity',
          'is_user_registered',
        ],
        changeMethods: [
          'register_user',
          'deposit',
          'withdraw',
          'transfer',
          'pause_pool',
          'unpause_pool',
          'emergency_withdraw',
        ],
      }
    );
  }

  /**
   * Register a user with phone number and NEAR account
   * SMS Command: JOIN <name>
   */
  async registerUser(phoneNumber: string, nearAccount: string): Promise<{
    success: boolean;
    txHash: string;
  }> {
    try {
      console.log(`📝 Registering user ${phoneNumber} → ${nearAccount}`);
      
      await this.initialize();
      
      const result = await this.contract.register_user({
        phone_number: phoneNumber,
        near_account: nearAccount,
      }, '300000000000000', // 0.3 NEAR gas
      '0'); // No deposit for registration

      console.log(`✅ User registered: ${result.transaction.hash}`);
      
      return {
        success: true,
        txHash: result.transaction.hash,
      };
    } catch (error: any) {
      console.error('❌ Register user error:', error.message);
      throw new Error(`Failed to register user: ${error.message}`);
    }
  }

  /**
   * Deposit NEAR tokens to the pool
   * SMS Command: DEPOSIT (handled by direct NEAR transfer)
   */
  async deposit(phoneNumber: string, amount: string): Promise<{
    success: boolean;
    txHash: string;
  }> {
    try {
      console.log(`💰 Depositing ${amount} NEAR for ${phoneNumber}`);
      
      const result = await this.contract.deposit({
        phone_number: phoneNumber,
      }, '300000000000000', // 0.3 NEAR gas
      nearApi.utils.format.parseNearAmount(amount)); // Deposit amount

      console.log(`✅ Deposit complete: ${result.transaction.hash}`);
      
      return {
        success: true,
        txHash: result.transaction.hash,
      };
    } catch (error: any) {
      console.error('❌ Deposit error:', error.message);
      throw new Error(`Failed to deposit: ${error.message}`);
    }
  }

  /**
   * Withdraw NEAR tokens from the pool
   * SMS Command: CASHOUT <amount> NEAR
   */
  async withdraw(phoneNumber: string, amount: string): Promise<{
    success: boolean;
    txHash: string;
  }> {
    try {
      console.log(`💸 Withdrawing ${amount} NEAR for ${phoneNumber}`);
      
      const result = await this.contract.withdraw({
        phone_number: phoneNumber,
        amount: nearApi.utils.format.parseNearAmount(amount),
      }, '300000000000000'); // 0.3 NEAR gas

      console.log(`✅ Withdrawal complete: ${result.transaction.hash}`);
      
      return {
      // Register user in TXTC token contract before redeeming
      const txtcTokenContract = process.env.TXTC_TOKEN_CONTRACT || 'txtc.token.primitives.testnet';
      try {
        console.log(`📝 Registering ${userAccount} in TXTC token...`);
        await this.account.functionCall({
          contractId: txtcTokenContract,
          methodName: 'storage_deposit',
          args: { account_id: userAccount, registration_only: true },
          gas: BigInt('30000000000000'),
          attachedDeposit: BigInt('1250000000000000000000'),
        });
        console.log(`✅ TX registered`);
      } catch (e) {
        console.warn(`⚠️  Storage: ${e.message}`);
      }

      
      const result = await this.contract.transfer({
        from_phone: fromPhone,
        to_phone: toPhone,
        amount: nearApi.utils.format.parseNearAmount(amount),
      }, '300000000000000'); // 0.3 NEAR gas

      console.log(`✅ Transfer complete: ${result.transaction.hash}`);
      
      return {
        success: true,
        txHash: result.transaction.hash,
      };
    } catch (error: any) {
      console.error('❌ Transfer error:', error.message);
      throw new Error(`Failed to transfer: ${error.message}`);
    }
  }

  /**
   * Get user balance
   * SMS Command: BALANCE
   */
  async getUserBalance(phoneNumber: string): Promise<{
    balance: string;
    nearAccount?: string;
  }> {
    try {
      console.log(`📊 Getting balance for ${phoneNumber}`);
      
      const result = await this.contract.get_balance({
        phone_number: phoneNumber,
      });

      return {
        balance: nearApi.utils.format.formatNearAmount(result.balance),
        nearAccount: result.near_account,
      };
    } catch (error: any) {
      console.error('❌ Get balance error:', error.message);
      throw new Error(`Failed to get balance: ${error.message}`);
    }
  }

  /**
   * Get total pool liquidity
   */
  async getTotalLiquidity(): Promise<string> {
    try {
      const result = await this.contract.get_total_liquidity();
      return nearApi.utils.format.formatNearAmount(result);
    } catch (error: any) {
      console.error('❌ Get total liquidity error:', error.message);
      throw new Error(`Failed to get total liquidity: ${error.message}`);
    }
  }

  /**
   * Check if user is registered
   */
  async isUserRegistered(phoneNumber: string): Promise<boolean> {
    try {
      const result = await this.contract.is_user_registered({
        phone_number: phoneNumber,
      });
      return result;
    } catch (error: any) {
      console.error('❌ Check registration error:', error.message);
      return false;
    }
  }

  /**
   * Send NEAR for gas fees to a user
   */
  async sendGasFee(toAccount: string, amount: string): Promise<string> {
    try {
      console.log(`⛽ Sending ${amount} NEAR gas fee to ${toAccount}`);
      
      const result = await this.account.sendMoney(
        toAccount,
        nearApi.utils.format.parseNearAmount(amount)
      );

      console.log(`✅ Gas fee sent: ${result.transaction.hash}`);
      return result.transaction.hash;
    } catch (error: any) {
      console.error('❌ Send gas fee error:', error.message);
      throw new Error(`Failed to send gas fee: ${error.message}`);
    }
  }

  /**
   * Create NEAR account for user (if needed)
   */
  async createAccount(accountId: string, publicKey: string): Promise<{
    success: boolean;
    txHash: string;
  }> {
    try {
      console.log(`👤 Creating NEAR account: ${accountId}`);
      
      const result = await this.account.createAccount(
        accountId,
        publicKey,
        nearApi.utils.format.parseNearAmount('0.1') // Initial balance
      );

      console.log(`✅ Account created: ${result.transaction.hash}`);
      
      return {
        success: true,
        txHash: result.transaction.hash,
      };
    } catch (error: any) {
      console.error('❌ Create account error:', error.message);
      throw new Error(`Failed to create account: ${error.message}`);
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
