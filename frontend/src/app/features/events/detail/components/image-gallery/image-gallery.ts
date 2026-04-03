import { CommonModule } from '@angular/common';
import { Component, EventEmitter, Input, Output } from '@angular/core';

@Component({
  selector: 'app-image-gallery',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './image-gallery.html',
  styleUrl: './image-gallery.scss',
})
export class ImageGallery {
  @Input() images: string[] = [];
  @Input() title = 'Event gallery';
  @Output() imageExpanded = new EventEmitter<string>();

  trackImage(index: number, image: string): string {
    return `${index}-${image}`;
  }
}
