import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { Router, RouterModule } from '@angular/router';
import { AuthService, UserRole } from '../../../core/auth/auth';

@Component({
  selector: 'app-register',
  standalone: true,
  imports: [CommonModule, FormsModule, RouterModule],
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
  
  error = '';

  constructor(private authService: AuthService, private router: Router) {}

  onSubmit() {
    this.error = '';

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
        if (err.status === 409) {
          this.error = 'That email is already in use. Please log in.';
        } else if (err.status === 401 || err.status === 403) {
          this.error = 'Invalid email or password.';
        } else {
          // If the backend returned a plain text string, try to use it, else default
          this.error = (typeof err.error === 'string' ? err.error : err.error?.message) || 'Failed to register. Please try again.';
        }
      }
    });
  }
}
