import { Box, Text } from '@mantine/core';
import { useQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import { IconCircleDot } from '@tabler/icons-react';
import { WeekProgress } from '../../features/planner/components/WeekProgress';
import { AccountButton } from '../../features/sync/components/AccountButton';
import { SyncChip } from '../../features/sync/components/SyncChip';
import { syncStatusQueryOptions } from '../../features/sync/queries';
import { navItems } from '../Layout/navItems';
import classes from './NavbarSearch.module.css';

export function NavbarSearch() {
  const { data: status } = useQuery(syncStatusQueryOptions);

  const mainLinks = navItems(status?.mode === 'signed_in').map((link) => (
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
