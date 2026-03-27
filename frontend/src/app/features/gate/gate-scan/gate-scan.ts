import { Component, inject, OnInit, ViewChild } from '@angular/core';
import { CommonModule } from '@angular/common';
import { NgxScannerQrcodeComponent, ScannerQRCodeConfig } from 'ngx-scanner-qrcode';
import { ToastrService } from 'ngx-toastr';
import { MatCardModule } from '@angular/material/card';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatSelectModule } from '@angular/material/select';
import { FormsModule } from '@angular/forms';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';

import { GateService, ScanLog, ScanResultResponse } from '../../../core/services/gate.service';
import { Event } from '../../../shared/models/event';

@Component({
  selector: 'app-gate-scan',
  standalone: true,
  imports: [
    CommonModule,
    NgxScannerQrcodeComponent,
    MatCardModule,
    MatButtonModule,
    MatIconModule,
    MatSelectModule,
    FormsModule,
    MatProgressSpinnerModule
  ],
  templateUrl: './gate-scan.html',
  styleUrls: ['./gate-scan.scss']
})
export class GateScan implements OnInit {
  @ViewChild('action') scanner!: NgxScannerQrcodeComponent;

  readonly scannerConfig: ScannerQRCodeConfig = {
    isBeep: true,
    constraints: {
      audio: false,
      video: {
        facingMode: {
          ideal: 'environment',
        },
      },
    },
  };

  events: Event[] = [];
  selectedEventId: string | null = null;
  scanHistory: ScanLog[] = [];
  
  isProcessing = false;
  lastResult: ScanResultResponse | null = null;

  private readonly gateService = inject(GateService);
  private readonly toastr = inject(ToastrService);

  ngOnInit() {
    this.loadEvents();
  }

  loadEvents() {
    this.gateService.getAssignedEvents().subscribe({
      next: (events) => {
        this.events = events;
        if (this.events.length > 0) {
          this.selectedEventId = this.events[0].id;
          this.loadHistory();
        }
      },
      error: () => this.toastr.error('Failed to load assigned events')
    });
  }

  onEventChange() {
    this.lastResult = null;
    this.loadHistory();
  }

  loadHistory() {
    if (!this.selectedEventId) return;
    this.gateService.getScanHistory(this.selectedEventId).subscribe({
      next: (logs) => this.scanHistory = logs,
      error: () => this.toastr.error('Failed to load scan history')
    });
  }

  onScan(event: any) {
    if (!this.selectedEventId || this.isProcessing) return;
    
    // Ngx scanner returns an array of results or a single result string depending on version/config
    let payload = '';
    if (Array.isArray(event) && event.length > 0) {
      payload = event[0].value;
    } else if (typeof event === 'string') {
      payload = event;
    }

    if (!payload) return;

    // Pause scanner to give user time to read the result
    this.scanner.pause();
    this.isProcessing = true;

    this.gateService.validateTicket(payload, this.selectedEventId).subscribe({
      next: (res) => {
        this.lastResult = res;
        this.loadHistory();
        
        if (res.success) {
          this.toastr.success(res.message, 'Valid Ticket');
        } else {
          this.toastr.error(res.message, 'Invalid Ticket');
        }
        
        // Auto resume after 3 seconds
        setTimeout(() => {
          this.isProcessing = false;
          this.scanner.play();
        }, 3000);
      },
      error: (err) => {
        this.toastr.error('Server error during validation');
        this.isProcessing = false;
        setTimeout(() => this.scanner.play(), 2000);
      }
    });
  }
}
