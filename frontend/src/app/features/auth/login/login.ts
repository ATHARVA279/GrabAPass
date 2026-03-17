import { Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';
import { ToastrService } from 'ngx-toastr';

import { MatCardModule } from '@angular/material/card';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';

import { AuthService, UserRole } from '../../../core/auth/auth';

@Component({
  selector: 'app-login',
  standalone: true,
  imports: [
    CommonModule, 
    FormsModule, 
    RouterModule,
    MatCardModule,
    MatFormFieldModule,
    MatInputModule,
    MatButtonModule,
    MatIconModule
  ],
  templateUrl: './login.html',
  styleUrls: ['./login.scss']
})
export class Login {
  email = '';
  password = '';

  private readonly authService = inject(AuthService);
  private readonly router = inject(Router);
  private readonly route = inject(ActivatedRoute);
  private readonly toastr = inject(ToastrService);

  onSubmit() {
    this.authService.login({ email: this.email, password: this.password }).subscribe({
      next: async (res) => {
        const returnUrl = this.route.snapshot.queryParamMap.get('returnUrl');
        const targetUrl = returnUrl || this.authService.getDefaultRouteForRole(res.user.role);
        const navigated = await this.router.navigateByUrl(targetUrl);

        if (!navigated) {
          this.toastr.error('Login succeeded, but navigation failed.', 'Navigation Error');
        }
      },
      error: (err) => {
        const msg = err.status === 401 || err.status === 403
          ? 'Invalid email or password.'
          : (typeof err.error === 'string' ? err.error : err.error?.message) || 'Failed to login. Please try again.';
        this.toastr.error(msg, 'Login Failed');
      }
    });
  }
}
