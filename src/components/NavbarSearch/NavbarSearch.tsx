import { IconCalendarWeek, IconCircleDot, IconLink, IconUsers } from '@tabler/icons-react';
import { Box, Text } from '@mantine/core';
import { useQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import { WeekProgress } from '../../features/planner/components/WeekProgress';
import { AccountButton } from '../../features/sync/components/AccountButton';
import { SyncChip } from '../../features/sync/components/SyncChip';
import { syncStatusQueryOptions } from '../../features/sync/queries';
import classes from './NavbarSearch.module.css';

type NavTo = '/' | '/recipes' | '/family';

const links: { icon: typeof IconCalendarWeek; label: string; to: NavTo }[] = [
  { icon: IconCalendarWeek, label: 'Schedule', to: '/' },
  { icon: IconLink, label: 'Recipe links', to: '/recipes' },
];

export function NavbarSearch() {
  const { data: status } = useQuery(syncStatusQueryOptions);
  // Family only exists for signed-in accounts, so the item is hidden until then.
  const familyLink = status?.mode === 'signed_in'
    ? { icon: IconUsers, label: 'Family', to: '/family' as NavTo }
    : null;
  const allLinks = familyLink ? [...links, familyLink] : links;

  const mainLinks = allLinks.map((link) => (
    <Link
      key={link.label}
      to={link.to}
      activeOptions={{ exact: link.to === '/' }}
      className={classes.mainLink}
      activeProps={{ className: `${classes.mainLink} ${classes.mainLinkActive}` }}
    >
      <link.icon size={20} className={classes.mainLinkIcon} stroke={1.5} />
      <span>{link.label}</span>
    </Link>
  ));

  return (
    <nav className={classes.navbar}>
      <div className={classes.brand}>
        <Box className={classes.logo}>
          <IconCircleDot size={16} stroke={2} />
        </Box>
        <Text fw={700} size="lg">Mise</Text>
      </div>

      <div className={classes.mainLinks}>{mainLinks}</div>

      <div className={classes.footer}>
        <WeekProgress />
        <div className={classes.sync}>
          <SyncChip />
          <AccountButton />
        </div>
      </div>
    </nav>
  );
}
