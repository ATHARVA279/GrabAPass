import { Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { Router, RouterModule } from '@angular/router';
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
export class Register {
  name = '';
  email = '';
  phone_number = '';
  password = '';
  role: UserRole = UserRole.Customer;
  organizer_company = '';

  private readonly authService = inject(AuthService);
  private readonly router = inject(Router);
  private readonly toastr = inject(ToastrService);

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
      next: (res) => {
        if (res.user.role === UserRole.Organizer) {
          this.router.navigate(['/organizer']);
        } else if (res.user.role === UserRole.GateStaff) {
          this.router.navigate(['/gate']);
        } else {
          this.router.navigate(['/events']);
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
