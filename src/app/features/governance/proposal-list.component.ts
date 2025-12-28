import { Component, EventEmitter, OnInit, Output, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormBuilder, FormGroup, ReactiveFormsModule } from '@angular/forms';
import { GovernanceService } from '../../core/services/governance.service';
import { ProposalSummary } from '../../core/types/wallet.types';

@Component({
  selector: 'app-proposal-list',
  standalone: true,
  imports: [CommonModule, ReactiveFormsModule],
  templateUrl: './proposal-list.html',
  styleUrl: './proposal-list.scss'
})
export class ProposalListComponent implements OnInit {
  @Output() proposalSelected = new EventEmitter<number>();
  private governanceService = inject(GovernanceService);
  private formBuilder = inject(FormBuilder);

  // Data
  proposals: ProposalSummary[] = [];
  filteredProposals: ProposalSummary[] = [];

  // UI State
  isLoading = false;
  selectedProposal: ProposalSummary | null = null;

  // Filters
  filterForm: FormGroup;

  constructor() {
    this.filterForm = this.formBuilder.group({
      state: [''],
      proposer: [''],
      search: ['']
    });
  }

  async ngOnInit() {
    await this.loadProposals();
    this.setupFilters();
  }

  private async loadProposals() {
    this.isLoading = true;
    try {
      this.governanceService.getProposals().subscribe({
        next: (proposals) => {
          this.proposals = proposals;
          this.applyFilters();
          this.isLoading = false;
        },
        error: (error) => {
          console.error('Failed to load proposals:', error);
          this.isLoading = false;
        }
      });
    } catch (error) {
      console.error('Failed to load proposals:', error);
      this.isLoading = false;
    }
  }

  private setupFilters() {
    this.filterForm.valueChanges.subscribe(() => {
      this.applyFilters();
    });
  }

  private applyFilters() {
    const filters = this.filterForm.value;
    this.filteredProposals = this.proposals.filter(proposal => {
      // State filter
      if (filters.state && proposal.state.toLowerCase() !== filters.state.toLowerCase()) {
        return false;
      }

      // Proposer filter
      if (filters.proposer && !proposal.proposer.toLowerCase().includes(filters.proposer.toLowerCase())) {
        return false;
      }

      // Search filter (searches in description)
      if (filters.search && !proposal.description.toLowerCase().includes(filters.search.toLowerCase())) {
        return false;
      }

      return true;
    });
  }

  selectProposal(proposal: ProposalSummary) {
    this.selectedProposal = proposal;
    this.proposalSelected.emit(proposal.proposal_id);
  }

  clearSelection() {
    this.selectedProposal = null;
  }

  getStateBadgeClass(state: string): string {
    switch (state.toLowerCase()) {
      case 'active': return 'badge-active';
      case 'pending': return 'badge-pending';
      case 'succeeded': return 'badge-succeeded';
      case 'defeated': return 'badge-defeated';
      case 'queued': return 'badge-queued';
      case 'executed': return 'badge-executed';
      case 'cancelled': return 'badge-cancelled';
      case 'vetoed': return 'badge-vetoed';
      case 'expired': return 'badge-expired';
      default: return 'badge-default';
    }
  }

  getQuorumProgress(proposal: ProposalSummary): number {
    const totalVotes = proposal.votes_for + proposal.votes_against + proposal.votes_abstain;
    if (totalVotes === 0) return 0;

    // Assuming 4% quorum for demonstration
    const quorumThreshold = 4000000; // 4% of 100M supply
    return Math.min((totalVotes / quorumThreshold) * 100, 100);
  }

  getSupportPercentage(proposal: ProposalSummary): number {
    const totalVotes = proposal.votes_for + proposal.votes_against + proposal.votes_abstain;
    if (totalVotes === 0) return 0;
    return (proposal.votes_for / totalVotes) * 100;
  }

  isProposalActive(proposal: ProposalSummary): boolean {
    return this.governanceService.isProposalActive(proposal.vote_start, proposal.vote_end);
  }

  getTimeRemaining(proposal: ProposalSummary): string {
    const now = Math.floor(Date.now() / 1000);
    if (now < proposal.vote_start) {
      return `Starts in ${this.governanceService.getTimeUntilVoting(proposal.vote_start)}`;
    } else if (now <= proposal.vote_end) {
      const remaining = proposal.vote_end - now;
      const days = Math.floor(remaining / 86400);
      const hours = Math.floor((remaining % 86400) / 3600);
      if (days > 0) return `${days}d ${hours}h left`;
      return `${hours}h left`;
    } else {
      return 'Voting ended';
    }
  }

  truncateAddress(address: string): string {
    if (address.length <= 12) return address;
    return address.substring(0, 6) + '...' + address.substring(address.length - 4);
  }

  truncateDescription(description: string, maxLength: number = 150): string {
    if (description.length <= maxLength) return description;
    return description.substring(0, maxLength) + '...';
  }

  refresh() {
    this.loadProposals();
  }
}