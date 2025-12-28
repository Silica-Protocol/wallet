import { ComponentFixture, TestBed } from '@angular/core/testing';

import { GovernanceComponent } from './governance';

describe('GovernanceComponent', () => {
  let component: GovernanceComponent;
  let fixture: ComponentFixture<GovernanceComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [GovernanceComponent]
    })
    .compileComponents();

    fixture = TestBed.createComponent(GovernanceComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
