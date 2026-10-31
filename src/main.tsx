import '@mantine/core/styles.css';
import '@mantine/dates/styles.css';
import '@mantine/notifications/styles.css';
// ‼️ import schedule styles after core and dates package styles
import '@mantine/schedule/styles.css';
import { StrictMode } from "react";
import ReactDOM from "react-dom/client";
import { RouterProvider } from "@tanstack/react-router";
import { QueryClientProvider } from "@tanstack/react-query";
import { QueryClientAtomProvider } from "jotai-tanstack-query/react";
import { router } from "./router";
import { queryClient } from "./lib/queryClient";
import { createTheme, MantineColorsTuple, MantineProvider } from "@mantine/core";
import { ModalsProvider } from "@mantine/modals";
import { listen } from "@tauri-apps/api/event";
import { Notifications } from "@mantine/notifications";
import { plannerModals } from "./features/planner/modals/registry";

const myColor: MantineColorsTuple = [
  '#f9ffe1',
  '#f4ffcb',
  '#e8fe9a',
  '#dcfd65',
  '#cffc2d',
  '#cbfc1b',
  '#c7fc04',
  '#afe000',
  '#9ac700',
  '#83ac00'
];

// Grays from the design mockups. Index 7 is the page background (Mantine's body color in dark mode).
const darkColors: MantineColorsTuple = [
  '#e6e8eb',
  '#b4bac2',
  '#7d848e',
  '#2e3338',
  '#23272c',
  '#1c1f23',
  '#181b1e',
  '#14161a',
  '#101215',
  '#0d0e11',
];

const theme = createTheme({
  colors: {
    myColor,
    dark: darkColors,
  },
  primaryColor: 'myColor',
  components: {
    Modal: {
      styles: {
        content: { backgroundColor: 'var(--mantine-color-dark-5)' },
      },
    },
  },
});

const rootElement = document.getElementById('root');
if (!rootElement) throw new Error("No root element to mount to!");

if (!rootElement.innerHTML) {
  // The sync worker emits this after pulled changes are applied to the local database.
  void listen("sync://changed", () => queryClient.invalidateQueries());

  const root = ReactDOM.createRoot(rootElement)
  root.render(
		<StrictMode>
			<QueryClientProvider client={queryClient}>
				<QueryClientAtomProvider client={queryClient}>
				<MantineProvider theme={theme} defaultColorScheme="dark">
					<ModalsProvider modals={plannerModals}>
						<Notifications />
						<RouterProvider router={router} />
					</ModalsProvider>
				</MantineProvider>
				</QueryClientAtomProvider>
			</QueryClientProvider>
    </StrictMode>,
  )
}
