import { Injectable, NgZone, inject } from '@angular/core';
import { Observable, Subject } from 'rxjs';
import { apiUrl } from '../api/api-url';

@Injectable({
  providedIn: 'root'
})
export class WsService {
  private socket: WebSocket | null = null;
  private messageSubject = new Subject<any>();
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private currentEventId: string | null = null;
  private readonly zone = inject(NgZone);

  connectToEvent(eventId: string): Observable<any> {
    if (this.socket && this.currentEventId === eventId) {
      if (this.socket.readyState === WebSocket.OPEN) {
        return this.messageSubject.asObservable();
      }
    }
    
    this.disconnect();
    this.currentEventId = eventId;
    this.reconnectAttempts = 0;
    this.setupSocket();
    
    return this.messageSubject.asObservable();
  }

  private setupSocket(): void {
    if (!this.currentEventId) return;

    let baseUrl = apiUrl('');
    if (baseUrl === '') {
      const isSecure = window.location.protocol === 'https:';
      baseUrl = `${isSecure ? 'wss:' : 'ws:'}//${window.location.host}`;
    } else {
      if (baseUrl.startsWith('http://')) {
        baseUrl = baseUrl.replace('http://', 'ws://');
      } else if (baseUrl.startsWith('https://')) {
        baseUrl = baseUrl.replace('https://', 'wss://');
      } else if (baseUrl.startsWith('/')) { 
        const isSecure = window.location.protocol === 'https:';
        baseUrl = `${isSecure ? 'wss:' : 'ws:'}//${window.location.host}${baseUrl.replace(/\/$/, '')}`;
      }
    }

    const wsUrl = `${baseUrl}/api/events/${this.currentEventId}/ws`;
    this.socket = new WebSocket(wsUrl);

    this.socket.onopen = () => {
      this.reconnectAttempts = 0;
    };

    this.socket.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        this.zone.run(() => {
          this.messageSubject.next(data);
        });
      } catch (e) {
        console.error('Invalid WS message payload', e);
      }
    };

    this.socket.onclose = () => {
      this.attemptReconnect();
    };

    this.socket.onerror = (error) => {
      console.error('WebSocket error:', error);
      this.socket?.close();
    };
  }

  private attemptReconnect(): void {
    if (this.reconnectAttempts < this.maxReconnectAttempts) {
      this.reconnectAttempts++;
      const timeout = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 10000);
      setTimeout(() => this.setupSocket(), timeout);
    }
  }

  disconnect(): void {
    if (this.socket) {
      this.socket.onclose = null; // prevent reconnect loop
      this.socket.close();
      this.socket = null;
    }
    this.currentEventId = null;
  }
}
