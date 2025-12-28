import { ComponentFixture, TestBed } from '@angular/core/testing';

import { StakingComponent } from './staking';

describe('StakingComponent', () => {
  let component: StakingComponent;
  let fixture: ComponentFixture<StakingComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [StakingComponent]
    })
    .compileComponents();

    fixture = TestBed.createComponent(StakingComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
