import { Component, OnInit, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormBuilder, FormGroup, FormArray, ReactiveFormsModule, Validators } from '@angular/forms';
import { GovernanceService } from '../../core/services/governance.service';

@Component({
  selector: 'app-create-proposal',
  standalone: true,
  imports: [CommonModule, ReactiveFormsModule],
  templateUrl: './create-proposal.html',
  styleUrl: './create-proposal.scss'
})
export class CreateProposalComponent implements OnInit {
  private governanceService = inject(GovernanceService);
  private formBuilder = inject(FormBuilder);

  // Form
  proposalForm: FormGroup;

  // UI State
  isSubmitting = false;
  currentStep = 1; // 1: Basic Info, 2: Actions, 3: Review

  constructor() {
    this.proposalForm = this.formBuilder.group({
      title: ['', [Validators.required, Validators.minLength(10), Validators.maxLength(100)]],
      description: ['', [Validators.required, Validators.minLength(50), Validators.maxLength(10000)]],
      actions: this.formBuilder.array([])
    });
  }

  ngOnInit() {
    // Add at least one action by default
    this.addAction();
  }

  get actions(): FormArray {
    return this.proposalForm.get('actions') as FormArray;
  }

  addAction() {
    const actionGroup = this.formBuilder.group({
      target: ['', [Validators.required, Validators.pattern(/^chert_[a-zA-Z0-9]+$/)]],
      value: ['0', [Validators.required, Validators.min(0)]],
      calldata: ['', Validators.required]
    });

    this.actions.push(actionGroup);
  }

  removeAction(index: number) {
    if (this.actions.length > 1) {
      this.actions.removeAt(index);
    }
  }

  nextStep() {
    if (this.currentStep < 3) {
      this.currentStep++;
    }
  }

  prevStep() {
    if (this.currentStep > 1) {
      this.currentStep--;
    }
  }

  canProceedToNext(): boolean {
    switch (this.currentStep) {
      case 1:
        return !!(this.proposalForm.get('title')?.valid && this.proposalForm.get('description')?.valid);
      case 2:
        return this.actions.valid && this.actions.length > 0;
      default:
        return true;
    }
  }

  async createProposal() {
     if (this.proposalForm.invalid) return;

     this.isSubmitting = true;
     try {
       const formValue = this.proposalForm.value;

       // TODO: Implement proposal creation API call
       console.log('Creating proposal:', formValue);

       // For now, show that governance is not yet implemented
       alert('Proposal creation not yet implemented. Governance features will be available soon.');

       // Don't reset form since creation failed
       // this.proposalForm.reset();
       // this.actions.clear();
       // this.addAction();
       // this.currentStep = 1;

     } catch (error) {
       console.error('Failed to create proposal:', error);
       alert('Failed to create proposal. Governance features may not be available yet.');
     } finally {
       this.isSubmitting = false;
     }
   }

  // Helper methods
  getActionSummary(action: { target?: string; value?: string }): string {
    const target = action.target || 'Unknown';
    const value = action.value || '0';
    return `Call ${target} with ${value} CHERT`;
  }

  getFormErrors(fieldName: string): string[] {
    const field = this.proposalForm.get(fieldName);
    if (field && field.errors && field.touched) {
      const errors = [];
      if (field.errors['required']) errors.push('This field is required');
      if (field.errors['minlength']) errors.push(`Minimum length is ${field.errors['minlength'].requiredLength}`);
      if (field.errors['maxlength']) errors.push(`Maximum length is ${field.errors['maxlength'].requiredLength}`);
      if (field.errors['pattern']) errors.push('Invalid format');
      return errors;
    }
    return [];
  }

  getActionErrors(index: number, fieldName: string): string[] {
    const action = this.actions.at(index);
    const field = action.get(fieldName);
    if (field && field.errors && field.touched) {
      const errors = [];
      if (field.errors['required']) errors.push('This field is required');
      if (field.errors['min']) errors.push('Value must be non-negative');
      if (field.errors['pattern']) errors.push('Invalid Chert address format');
      return errors;
    }
    return [];
  }
}