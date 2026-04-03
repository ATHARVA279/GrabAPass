import { CommonModule } from '@angular/common';
import { Component, Input } from '@angular/core';

import { EventVenueInput } from '../../models/event-venue';

@Component({
  selector: 'app-address-form-group',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './address-form-group.html',
  styleUrl: './address-form-group.scss',
})
export class AddressFormGroup {
  @Input() venue: EventVenueInput | null = null;
}
