import { Routes } from '@angular/router';
import { Login } from './features/auth/login/login';
import { Register } from './features/auth/register/register';
import { authGuard } from './core/auth/auth-guard';

export const routes: Routes = [
  { path: 'login', component: Login },
  { path: 'register', component: Register },
  {
    path: 'organizer',
    canActivate: [authGuard],
    data: { role: 'Organizer' },
    loadComponent: () => import('./features/events/organizer/dashboard/dashboard').then(m => m.Dashboard)
  },
  {
    path: 'organizer/create-event',
    canActivate: [authGuard],
    data: { role: 'Organizer' },
    loadComponent: () => import('./features/events/organizer/create-event/create-event').then(m => m.CreateEvent)
  },
  {
    path: 'organizer/create-venue',
    canActivate: [authGuard],
    data: { role: 'Organizer' },
    loadComponent: () => import('./features/venues/create-venue/create-venue').then(m => m.CreateVenue)
  },
  {
    path: 'gate',
    canActivate: [authGuard],
    data: { role: 'GateStaff' },
    loadComponent: () => import('./features/events/home/home').then(m => m.Home)
  },
  {
    path: 'events',
    loadComponent: () => import('./features/events/home/home').then(m => m.Home),
    pathMatch: 'full'
  },
  {
    path: 'events/:id',
    loadComponent: () => import('./features/events/detail/event-detail').then(m => m.EventDetail),
    pathMatch: 'full'
  },
  {
    path: 'events/:id/seats',
    loadComponent: () => import('./features/events/seat-selection/seat-selection').then(m => m.SeatSelection)
  },
  {
    path: 'events/:id/checkout',
    canActivate: [authGuard],
    data: { role: 'Customer' },
    loadComponent: () => import('./features/events/checkout/checkout').then(m => m.Checkout)
  },
  {
    path: 'orders',
    canActivate: [authGuard],
    data: { role: 'Customer' },
    loadComponent: () => import('./features/orders/order-list/order-list').then(m => m.OrderList)
  },
  {
    path: 'orders/:id/confirmation',
    canActivate: [authGuard],
    data: { role: 'Customer' },
    loadComponent: () => import('./features/orders/order-confirmation/order-confirmation').then(m => m.OrderConfirmation)
  },
  { 
    path: '', 
    redirectTo: '/events',
    pathMatch: 'full' 
  },
  {
    path: '**',
    redirectTo: '/events'
  }
];
