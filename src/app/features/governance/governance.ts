import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ProposalListComponent } from './proposal-list.component';
import { ProposalDetailComponent } from './proposal-detail.component';
import { VotingPowerComponent } from './voting-power.component';
import { CreateProposalComponent } from './create-proposal.component';

@Component({
  selector: 'app-governance',
  standalone: true,
  imports: [CommonModule, ProposalListComponent, ProposalDetailComponent, VotingPowerComponent, CreateProposalComponent],
  templateUrl: './governance.html',
  styleUrl: './governance.scss'
})
export class GovernanceComponent {
  // View state
  currentView: 'list' | 'detail' | 'voting-power' | 'create-proposal' = 'list';
  selectedProposalId: number | null = null;



  showProposalList() {
    this.currentView = 'list';
    this.selectedProposalId = null;
  }

  showProposalDetail(proposalId: number) {
    this.currentView = 'detail';
    this.selectedProposalId = proposalId;
  }

  showVotingPower() {
    this.currentView = 'voting-power';
    this.selectedProposalId = null;
  }

  showCreateProposal() {
    this.currentView = 'create-proposal';
    this.selectedProposalId = null;
  }

  onProposalSelected(proposalId: number) {
    this.showProposalDetail(proposalId);
  }
}
