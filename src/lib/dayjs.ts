// src/utils/dayjs.ts
import dayjs from 'dayjs';
import isoWeek from 'dayjs/plugin/isoWeek';

// Extend once globally for this instance
dayjs.extend(isoWeek);

// Export the configured dayjs
export default dayjs;
