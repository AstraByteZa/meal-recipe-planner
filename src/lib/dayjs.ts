// src/utils/dayjs.ts
import dayjs from 'dayjs';
import isoWeek from 'dayjs/plugin/isoWeek';
import relativeTime from 'dayjs/plugin/relativeTime';

// Extend once globally for this instance
dayjs.extend(isoWeek);
dayjs.extend(relativeTime);

// Export the configured dayjs
export default dayjs;
