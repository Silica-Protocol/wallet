import { Routes } from '@angular/router';

export const routes: Routes = [
  {
    path: '',
    redirectTo: '/wallet',
    pathMatch: 'full'
  },
  {
    path: 'wallet',
    loadComponent: () => import('./features/wallet/wallet').then(m => m.WalletComponent)
  },
   {
     path: 'transactions',
     loadComponent: () => import('./features/transactions/transactions').then(m => m.TransactionsComponent)
   },
   {
     path: 'send',
     loadComponent: () => import('./features/transactions/send-transaction.component').then(m => m.SendTransactionComponent)
   },
   {
     path: 'receive',
     loadComponent: () => import('./features/transactions/receive-transaction.component').then(m => m.ReceiveTransactionComponent)
   },
  {
    path: 'staking',
    loadComponent: () => import('./features/staking/staking').then(m => m.StakingComponent)
  },
   {
     path: 'governance',
     loadComponent: () => import('./features/governance/governance').then(m => m.GovernanceComponent)
   },
   {
     path: 'settings',
     loadComponent: () => import('./features/settings/settings').then(m => m.SettingsComponent)
   },
   {
     path: '**',
     redirectTo: '/wallet'
   }
];
