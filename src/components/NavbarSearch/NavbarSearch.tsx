import { IconCalendarWeek, IconCircleDot, IconLink, IconSearch } from '@tabler/icons-react';
import { Box, Code, Text, TextInput } from '@mantine/core';
import { Link } from '@tanstack/react-router';
import { WeekProgress } from '../../features/planner/components/WeekProgress';
import { SyncButton } from '../../features/sync/components/SyncButton';
import classes from './NavbarSearch.module.css';

const links: { icon: typeof IconCalendarWeek; label: string; to: '/' | '/recipes' }[] = [
  { icon: IconCalendarWeek, label: 'Schedule', to: '/' },
  { icon: IconLink, label: 'Recipe links', to: '/recipes' },
];

export function NavbarSearch() {
  const mainLinks = links.map((link) => (
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

      <TextInput
        placeholder="Search"
        size="xs"
        leftSection={<IconSearch size={12} stroke={1.5} />}
        rightSectionWidth={70}
        rightSection={<Code className={classes.searchCode}>Ctrl + K</Code>}
        styles={{ section: { pointerEvents: 'none' } }}
        mb="md"
        aria-label="Search"
      />

      <div className={classes.mainLinks}>{mainLinks}</div>

      <div className={classes.footer}>
        <SyncButton />
        <WeekProgress />
      </div>
    </nav>
  );
}
