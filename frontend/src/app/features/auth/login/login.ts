import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { Router, RouterModule } from '@angular/router';
import { AuthService, UserRole } from '../../../core/auth/auth';

@Component({
  selector: 'app-login',
  standalone: true,
  imports: [CommonModule, FormsModule, RouterModule],
  templateUrl: './login.html',
  styleUrls: ['./login.scss']
})
export class Login {
  email = '';
  password = '';
  error = '';

  constructor(private authService: AuthService, private router: Router) {}

  onSubmit() {
    this.error = '';
    this.authService.login({ email: this.email, password: this.password }).subscribe({
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
        if (err.status === 401 || err.status === 403) {
          this.error = 'Invalid email or password.';
        } else {
          this.error = (typeof err.error === 'string' ? err.error : err.error?.message) || 'Failed to login. Please try again.';
        }
      }
    });
  }
}
