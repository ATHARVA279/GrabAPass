import { Component, inject, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';
import { ToastrService } from 'ngx-toastr';

import { MatCardModule } from '@angular/material/card';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatSelectModule } from '@angular/material/select';

import { AuthService, UserRole } from '../../../core/auth/auth';

@Component({
  selector: 'app-register',
  standalone: true,
  imports: [
    CommonModule, 
    FormsModule, 
    RouterModule,
    MatCardModule,
    MatFormFieldModule,
    MatInputModule,
    MatButtonModule,
    MatIconModule,
    MatSelectModule
  ],
  templateUrl: './register.html',
  styleUrls: ['./register.scss']
})
export class Register implements OnInit {
  name = '';
  email = '';
  phone_number = '';
  password = '';
  role: UserRole = UserRole.Customer;
  organizer_company = '';
  returnUrlMessage: string | null = null;

  private readonly authService = inject(AuthService);
  private readonly router = inject(Router);
  private readonly route = inject(ActivatedRoute);
  private readonly toastr = inject(ToastrService);

  ngOnInit() {
    const returnUrl = this.route.snapshot.queryParamMap.get('returnUrl');
    if (returnUrl && returnUrl.includes('/events/')) {
      this.returnUrlMessage = 'Create an account to book your tickets.';
    } else if (returnUrl && returnUrl.includes('/split/')) {
      this.returnUrlMessage = 'Create an account to claim your split ticket.';
    }
  }

  onSubmit() {
    const payload: any = {
      name: this.name,
      email: this.email,
      password: this.password,
      role: this.role,
    };

    if (this.phone_number) {
      payload.phone_number = this.phone_number;
    }

    if (this.role === UserRole.Organizer && this.organizer_company) {
      payload.organizer_company = this.organizer_company;
    }

    this.authService.register(payload).subscribe({
      next: async (res) => {
        const returnUrl = this.route.snapshot.queryParamMap.get('returnUrl');
        const targetUrl = returnUrl || this.authService.getDefaultRouteForRole(res.user.role);
        const navigated = await this.router.navigateByUrl(targetUrl);

        if (!navigated) {
          this.toastr.error('Registration succeeded, but navigation failed.', 'Navigation Error');
        }
      },
      error: (err) => {
        const msg = err.status === 409
          ? 'That email is already in use. Please log in.'
          : err.status === 401 || err.status === 403
            ? 'Invalid credentials.'
            : (typeof err.error === 'string' ? err.error : err.error?.message) || 'Failed to register. Please try again.';
        this.toastr.error(msg, 'Registration Failed');
      }
    });
  }
}
