import { Component, OnInit, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormBuilder, FormGroup, ReactiveFormsModule, Validators } from '@angular/forms';
import { WALLET_BACKEND, WalletBackend } from '../../core/services/wallet-backend.interface';
import { WalletService } from '../../core/services/wallet.service';

interface ValidatorInfo {
  address: string;
  publicKey: string;
  networkKey: string;
  stake: number;
  stakeAmount: number;
  isActive: boolean;
  commissionRate: number;
  reputationScore: number;
  lastActivity: string;
  totalDelegated?: number;
  delegatorCount?: number;
}

interface DelegationInfo {
  delegator: string;
  validator: string;
  amount: number;
  shares: number;
  delegatedAt: string;
  rewardsClaimed: number;
}

interface RewardInfo {
  totalEarned: number;
  pendingRewards: number;
  currentAPY: number;
}

interface LockBoxRecord {
  account: string;
  amount: number;
  termMonths: number;
  lockedAt: string;
  unlockAt: string;
  multiplier: number;
  isActive: boolean;
}

interface AutoStakeRecord {
  account: string;
  balance: number;
  maturityTimestamp: string;
  isActive: boolean;
}

@Component({
  selector: 'app-staking',
  standalone: true,
  imports: [CommonModule, ReactiveFormsModule],
  templateUrl: './staking.html',
  styleUrl: './staking.scss'
})
export class StakingComponent implements OnInit {
  private walletBackend = inject<WalletBackend>(WALLET_BACKEND);
  private walletService = inject(WalletService);
  private formBuilder = inject(FormBuilder);

  // Data
  validators: ValidatorInfo[] = [];
  userDelegations: DelegationInfo[] = [];
  stakingRewards: RewardInfo = { totalEarned: 0, pendingRewards: 0, currentAPY: 0 };
  lockboxRecords: LockBoxRecord[] = [];
  autoStakeStatus: AutoStakeRecord | null = null;
  currentAddress = '';

  // UI State
  selectedTab: 'overview' | 'validators' | 'delegations' | 'rewards' = 'overview';
  isLoading = false;
  selectedValidator: ValidatorInfo | null = null;

  // Forms
  delegateForm: FormGroup;
  lockboxForm: FormGroup;

  constructor() {
    this.delegateForm = this.formBuilder.group({
      validatorAddress: ['', Validators.required],
      amount: ['', [Validators.required, Validators.min(1)]]
    });

    this.lockboxForm = this.formBuilder.group({
      amount: ['', [Validators.required, Validators.min(100)]],
      termMonths: ['3', Validators.required]
    });
  }

  async ngOnInit() {
    await this.loadWalletInfo();
    await this.loadStakingData();
  }

  private async loadWalletInfo() {
     try {
       const walletInfo = await this.walletBackend.getWalletInfo();
       if (walletInfo.exists && !walletInfo.isLocked) {
         // Get the current address from wallet service
         const address = this.walletService.createdAddress();
         if (address) {
           this.currentAddress = address;
         } else {
           throw new Error('Wallet address not available');
         }
       } else {
         throw new Error('Wallet not initialized or unlocked');
       }
     } catch (error) {
       console.error('Failed to load wallet info:', error);
       this.isLoading = false;
       // Show error to user
       alert('Failed to load wallet information. Please ensure your wallet is unlocked.');
     }
   }

  private async loadStakingData() {
    this.isLoading = true;
    try {
      await Promise.all([
        this.loadValidators(),
        this.loadUserDelegations(),
        this.loadStakingRewards(),
        this.loadLockboxRecords(),
        this.loadAutoStakeStatus()
      ]);
    } catch (error) {
      console.error('Failed to load staking data:', error);
    } finally {
      this.isLoading = false;
    }
  }

  private async loadValidators() {
     try {
       // Call the actual Tauri command for validators
       const response = await this.walletBackend.getValidators();
       this.validators = response.validators || [];
     } catch (error) {
       console.error('Failed to load validators:', error);
       this.validators = []; // Empty array as fallback
       // Could show user notification here
     }
   }

  private async loadUserDelegations() {
     try {
       // Call the actual Tauri command for user delegations
       const response = await this.walletBackend.getUserDelegations({ userAddress: this.currentAddress });
       this.userDelegations = response.delegations || [];
     } catch (error) {
       console.error('Failed to load user delegations:', error);
       this.userDelegations = []; // Empty array as fallback
     }
   }

  private async loadStakingRewards() {
     try {
       // Call the actual Tauri command for staking rewards
       const response = await this.walletBackend.getStakingRewards({ userAddress: this.currentAddress });
       this.stakingRewards = response.rewards;
     } catch (error) {
       console.error('Failed to load staking rewards:', error);
       // Keep default values as fallback
       this.stakingRewards = { totalEarned: 0, pendingRewards: 0, currentAPY: 0 };
     }
   }

  private async loadLockboxRecords() {
     try {
       // Call the actual Tauri command for lockbox records
       const response = await this.walletBackend.getLockboxRecords({ userAddress: this.currentAddress });
       this.lockboxRecords = response.records || [];
     } catch (error) {
       console.error('Failed to load lockbox records:', error);
       this.lockboxRecords = []; // Empty array as fallback
     }
   }

  private async loadAutoStakeStatus() {
     try {
       // Call the actual Tauri command for auto-stake status
       const response = await this.walletBackend.getAutoStakeStatus({ userAddress: this.currentAddress });
       this.autoStakeStatus = response.status;
     } catch (error) {
       console.error('Failed to load auto-stake status:', error);
       this.autoStakeStatus = null; // Null as fallback (disabled)
     }
   }

  selectTab(tab: 'overview' | 'validators' | 'delegations' | 'rewards') {
    this.selectedTab = tab;
  }

  openDelegateModal(validator: ValidatorInfo) {
    this.selectedValidator = validator;
    this.delegateForm.patchValue({
      validatorAddress: validator.address,
      amount: ''
    });
  }

  closeDelegateModal() {
    this.selectedValidator = null;
    this.delegateForm.reset();
  }

  async delegate() {
     if (this.delegateForm.invalid) return;

     const { validatorAddress, amount } = this.delegateForm.value;

     try {
       this.isLoading = true;
       // Call the actual Tauri command for delegation
       await this.walletBackend.delegateTokens({
         delegatorAddress: this.currentAddress,
         validatorAddress,
         amount: parseInt(amount)
       });

       // Reload data after successful delegation
       await this.loadStakingData();
       this.closeDelegateModal();
     } catch (error) {
       console.error('Failed to delegate:', error);
       alert('Failed to delegate tokens. Please try again.');
     } finally {
       this.isLoading = false;
     }
   }

  async undelegate(delegation: DelegationInfo, amount: number) {
     try {
       this.isLoading = true;
       // Call the actual Tauri command for undelegation
       await this.walletBackend.undelegateTokens({
         delegatorAddress: this.currentAddress,
         validatorAddress: delegation.validator,
         amount
       });

       // Reload data after successful undelegation
       await this.loadStakingData();
     } catch (error) {
       console.error('Failed to undelegate:', error);
       alert('Failed to undelegate tokens. Please try again.');
     } finally {
       this.isLoading = false;
     }
   }

  async createLockboxStake() {
     if (this.lockboxForm.invalid) return;

     const { amount, termMonths } = this.lockboxForm.value;

     try {
       this.isLoading = true;
       // Call the actual Tauri command for lockbox stake creation
       await this.walletBackend.createLockboxStake({
         account: this.currentAddress,
         amount: parseInt(amount),
         termMonths: parseInt(termMonths)
       });

       // Reload data after successful creation
       await this.loadStakingData();
       this.lockboxForm.reset();
     } catch (error) {
       console.error('Failed to create lockbox stake:', error);
       alert('Failed to create lockbox stake. Please try again.');
     } finally {
       this.isLoading = false;
     }
   }

  async toggleAutoStaking() {
     try {
       this.isLoading = true;
       const enable = !this.autoStakeStatus?.isActive;

       // Call the actual Tauri command for toggling auto-staking
       await this.walletBackend.toggleAutoStaking({
         account: this.currentAddress,
         enable
       });

       // Reload auto-stake status after toggle
       await this.loadAutoStakeStatus();
     } catch (error) {
       console.error('Failed to toggle auto-staking:', error);
       alert('Failed to toggle auto-staking. Please try again.');
     } finally {
       this.isLoading = false;
     }
   }

  async claimRewards() {
     try {
       this.isLoading = true;
       // Call the actual Tauri command for claiming rewards
       await this.walletBackend.claimStakingRewards({
         account: this.currentAddress
       });

       // Reload rewards data after claiming
       await this.loadStakingRewards();
     } catch (error) {
       console.error('Failed to claim rewards:', error);
       alert('Failed to claim staking rewards. Please try again.');
     } finally {
       this.isLoading = false;
     }
   }

  getTermMultiplier(termMonths: number): number {
    switch (termMonths) {
      case 1: return 1.05;
      case 3: return 1.15;
      case 6: return 1.30;
      case 12: return 1.60;
      default: return 1.0;
    }
  }

  getTermAPY(termMonths: number): number {
    switch (termMonths) {
      case 1: return 5;
      case 3: return 15;
      case 6: return 30;
      case 12: return 60;
      default: return 0;
    }
  }

  formatAmount(amount: number): string {
    return (amount / 1000000).toFixed(2) + ' CHERT';
  }

  truncateAddress(address: string): string {
    if (address.length <= 12) return address;
    return address.substring(0, 6) + '...' + address.substring(address.length - 4);
  }

  getLockboxProgress(record: LockBoxRecord): number {
    const now = new Date().getTime();
    const lockedAt = new Date(record.lockedAt).getTime();
    const unlockAt = new Date(record.unlockAt).getTime();
    const totalDuration = unlockAt - lockedAt;
    const elapsed = now - lockedAt;

    if (elapsed >= totalDuration) return 100;
    if (elapsed <= 0) return 0;

    return Math.round((elapsed / totalDuration) * 100);
  }

  getTotalStaked(): string {
    const total = this.userDelegations.reduce((sum, d) => sum + d.amount, 0);
    return this.formatAmount(total);
  }

  getEstimatedAPY(validator: ValidatorInfo): string {
    // Simple estimation: commission rate * 0.8 (accounting for network factors)
    return (validator.commissionRate * 0.8).toFixed(1);
  }
}
