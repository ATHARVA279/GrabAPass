export function getTicketStatusClass(status: string): string {
  switch (status.toLowerCase()) {
    case 'valid':
      return 'status-valid';
    case 'used':
      return 'status-used';
    case 'cancelled':
      return 'status-cancelled';
    default:
      return '';
  }
}
