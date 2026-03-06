import { Injectable } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { BehaviorSubject, Observable, tap } from 'rxjs';

export enum UserRole {
  Customer = 'Customer',
  Organizer = 'Organizer',
  GateStaff = 'GateStaff',
}

export interface User {
  id: string;
  email: string;
  role: UserRole;
  name: string;
}

export interface AuthResponse {
  token: string;
  user: User;
}

@Injectable({
  providedIn: 'root',
})
export class AuthService {
  private apiUrl = '/api/auth';
  private currentUserSubject = new BehaviorSubject<User | null>(null);
  public currentUser$ = this.currentUserSubject.asObservable();

  constructor(private http: HttpClient) {
    this.checkToken();
  }

  get currentUserValue(): User | null {
    return this.currentUserSubject.value;
  }

  login(credentials: { email: string; password: string }): Observable<AuthResponse> {
    return this.http.post<AuthResponse>(`${this.apiUrl}/login`, credentials).pipe(
      tap((res) => this.setSession(res))
    );
  }

  register(userData: {
    name: string;
    email: string;
    password: string;
    role: UserRole;
    phone_number?: string;
    organizer_company?: string;
  }): Observable<AuthResponse> {
    return this.http.post<AuthResponse>(`${this.apiUrl}/register`, userData).pipe(
      tap((res) => this.setSession(res))
    );
  }

  logout() {
    localStorage.removeItem('jwt_token');
    this.currentUserSubject.next(null);
  }

  getToken(): string | null {
    return localStorage.getItem('jwt_token');
  }

  private setSession(authResult: AuthResponse) {
    localStorage.setItem('jwt_token', authResult.token);
    this.currentUserSubject.next(authResult.user);
  }

  private checkToken() {
    const token = this.getToken();
    if (token) {
      // In a real app we would decode the JWT or fetch /me here.
      // For simplicity, we decode the role from the token payload.
      try {
        const payload = JSON.parse(atob(token.split('.')[1]));
        this.currentUserSubject.next({
          id: payload.sub,
          email: payload.email,
          role: payload.role as UserRole,
          name: payload.name || 'User',
        });
      } catch {
        this.logout();
      }
    }
  }
}
