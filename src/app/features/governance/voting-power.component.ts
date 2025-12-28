import { Component, OnInit, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormBuilder, FormGroup, ReactiveFormsModule, Validators } from '@angular/forms';
import { GovernanceService } from '../../core/services/governance.service';
import { VotingPowerInfo, DelegationInfo } from '../../core/types/wallet.types';

@Component({
  selector: 'app-voting-power',
  standalone: true,
  imports: [CommonModule, ReactiveFormsModule],
  templateUrl: './voting-power.html',
  styleUrl: './voting-power.scss'
})
export class VotingPowerComponent implements OnInit {
  private governanceService = inject(GovernanceService);
  private formBuilder = inject(FormBuilder);

  // Data
  votingPower: VotingPowerInfo | null = null;
  delegations: DelegationInfo[] = [];
  currentAddress: string | null = null;

  // UI State
  isLoading = false;
  selectedTab: 'overview' | 'delegations' = 'overview';

  // Forms
  addressForm: FormGroup;
  delegateForm: FormGroup;

  constructor() {
    this.addressForm = this.formBuilder.group({
      address: ['', [Validators.required, Validators.pattern(/^chert_[a-zA-Z0-9]+$/)]]
    });

    this.delegateForm = this.formBuilder.group({
      delegator: ['', [Validators.required, Validators.pattern(/^chert_[a-zA-Z0-9]+$/)]],
      validator: ['', [Validators.required, Validators.pattern(/^chert_[a-zA-Z0-9]+$/)]],
      amount: ['', [Validators.required, Validators.min(1)]]
    });
  }

  ngOnInit() {
    // Wait for user to supply address via form
  }

  private async loadVotingPower() {
    if (!this.currentAddress) {
      this.votingPower = null;
      return;
    }

    try {
      this.governanceService.getVotingPower(this.currentAddress).subscribe({
        next: (power) => {
          this.votingPower = power;
        },
        error: (error) => {
          console.error('Failed to load voting power:', error);
          alert('Failed to load voting power. Governance features may not be available yet.');
        }
      });
    } catch (error) {
      console.error('Failed to load voting power:', error);
      alert('Failed to load voting power. Please try again later.');
    }
  }

  private async loadDelegations() {
    if (!this.currentAddress) {
      this.delegations = [];
      return;
    }

    try {
      this.governanceService.getDelegations(this.currentAddress).subscribe({
        next: (delegations) => {
          this.delegations = delegations;
        },
        error: (error) => {
          console.error('Failed to load delegations:', error);
          alert('Failed to load delegations. Governance features may not be available yet.');
        }
      });
    } catch (error) {
      console.error('Failed to load delegations:', error);
      alert('Failed to load delegations. Please try again later.');
    }
  }

  applyAddress() {
    if (this.addressForm.invalid) {
      this.addressForm.markAllAsTouched();
      return;
    }

    this.currentAddress = this.addressForm.get('address')?.value;
    this.delegateForm.patchValue({ delegator: this.currentAddress });
    this.refresh();
  }

  selectTab(tab: 'overview' | 'delegations') {
    this.selectedTab = tab;
  }

  async delegate() {
    if (this.delegateForm.invalid) return;

    const { delegator, validator, amount } = this.delegateForm.value;
    if (!delegator) {
      this.delegateForm.get('delegator')?.markAsTouched();
      return;
    }

    try {
      this.isLoading = true;
      await this.governanceService.delegate({
        delegator,
        validator,
        amount
      }).toPromise();

      // Reload data after delegation
      await this.loadVotingPower();
      await this.loadDelegations();
      this.delegateForm.reset();
    } catch (error) {
      console.error('Failed to delegate:', error);
    } finally {
      this.isLoading = false;
    }
  }

  async undelegate(delegation: DelegationInfo) {
    try {
      // Placeholder until governance exposes an undelegation endpoint.
      console.log('Undelegate:', delegation);
      alert('Undelegation not yet implemented. This feature will be available soon.');
    } catch (error) {
      console.error('Failed to undelegate:', error);
      alert('Failed to undelegate. Please try again later.');
    }
  }

  getTotalDelegated(): number {
    return this.delegations.reduce((total, d) => total + d.amount, 0);
  }

  getAvailablePower(): number {
    if (!this.votingPower) return 0;
    return this.votingPower.voting_power - this.getTotalDelegated();
  }

  formatTimestamp(timestamp: number): string {
    return this.governanceService.formatTimestamp(timestamp);
  }

  truncateAddress(address: string): string {
    if (address.length <= 12) return address;
    return address.substring(0, 6) + '...' + address.substring(address.length - 4);
  }

  refresh() {
    this.loadVotingPower();
    this.loadDelegations();
  }
}