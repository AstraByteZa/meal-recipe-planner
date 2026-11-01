import {
  createRouter,
  createRoute,
  createRootRouteWithContext,
  Outlet,
  redirect,
  useParams,
  useRouterState,
  useSearch,
} from '@tanstack/react-router'
import { TanStackRouterDevtools } from '@tanstack/react-router-devtools'
import type { QueryClient } from '@tanstack/react-query'
import { Alert } from '@mantine/core'
import { DashboardLayout } from './components/Layout/DashboardLayout'
import { RouteLoader } from './components/RouteLoader/RouteLoader'
import { MealPlannerView } from './features/planner/views/MealPlannerView'
import { currentWeek, toWeekStart } from './features/planner/week'
import { scheduleQueryOptions, tracksQueryOptions } from './features/planner/queries'
import { recipeQueryOptions, recipesQueryOptions } from './features/recipes/queries'
import { RecipeLinksView } from './features/recipes/views/RecipeLinksView'
import { RecipeDetailView } from './features/recipes/views/RecipeDetailView'
import { RecipeEditView } from './features/recipes/views/RecipeEditView'
import { CookModeView } from './features/recipes/views/CookModeView'
import { toCookStep } from './features/recipes/cook/steps'
import { OnboardingView } from './features/onboarding/views/OnboardingView'
import { FamilyView } from './features/family/views/FamilyView'
import { SignInView } from './features/sync/views/SignInView'
import { JoinView } from './features/sync/views/JoinView'
import { syncStatusQueryOptions } from './features/sync/queries'
import { toNextPath } from './features/sync/nextPath'
import { toStep } from './features/onboarding/steps'
import { settingsQueryOptions } from './features/settings/queries'
import { configQueryOptions } from './features/config/queries'
import { queryClient } from './lib/queryClient'

interface RouterContext {
  queryClient: QueryClient
}

const rootRoute = createRootRouteWithContext<RouterContext>()({
  // Until onboarding is finished, every page except onboarding redirects to it.
  beforeLoad: async ({ context, location }) => {
    // Sign-in and invites work before onboarding, so a new device can join a family first.
    if (isOwnLayoutPath(location.pathname)) return
    // Config is cached for the session, so every page below can read it without waiting.
    await context.queryClient.query(configQueryOptions)
    const settings = await context.queryClient.query(settingsQueryOptions)
    if (!settings.onboarded) throw redirect({ to: '/onboarding', search: { step: 1 } })
  },
  component: RootLayout,
})

// Onboarding, sign-in, invites and cook mode have their own full-screen layouts.
// Every other page sits inside the app navbar.
function isOwnLayoutPath(pathname: string) {
  return pathname.startsWith('/onboarding') || pathname === '/sign-in' || pathname === '/join'
}

function RootLayout() {
  const pathname = useRouterState({ select: (state) => state.location.pathname })
  const hasOwnLayout = isOwnLayoutPath(pathname) || /^\/recipes\/[^/]+\/cook$/.test(pathname)

  return (
    <>
      {hasOwnLayout ? <Outlet /> : <DashboardLayout />}
      <TanStackRouterDevtools />
    </>
  )
}

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  // The shown week lives in the URL so the loader can fetch it.
  validateSearch: (search: Record<string, unknown>) => ({ week: toWeekStart(search.week) }),
  loaderDeps: ({ search }) => ({ week: search.week }),
  loader: ({ context, deps }) => Promise.all([
    context.queryClient.query(tracksQueryOptions),
    context.queryClient.query(scheduleQueryOptions(deps.week)),
  ]),
  pendingComponent: RouteLoader,
  errorComponent: ({ error }) => <Alert color="red" m="md" title="Could not load the schedule">{String(error)}</Alert>,
  component: MealPlannerView,
})

const aboutRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/about',
  component: function About() {
    return <div className="p-2">Hello from About!</div>
  },
})

const recipeLinksRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/recipes',
  // Fills the cache before rendering. The view reads it with useSuspenseQuery.
  loader: ({ context }) => context.queryClient.query(recipesQueryOptions),
  pendingComponent: RouteLoader,
  errorComponent: ({ error }) => <Alert color="red" m="md" title="Could not load recipes">{String(error)}</Alert>,
  component: RecipeLinksView,
})

// Both recipe pages load the recipe and the tracks they show before rendering.
const recipeLoader = ({ context, params }: { context: RouterContext; params: { recipeId: string } }) => Promise.all([
  context.queryClient.query(recipeQueryOptions(params.recipeId)),
  context.queryClient.query(tracksQueryOptions),
])

function RecipeDetailPage() {
  const { recipeId } = useParams({ from: '/recipes/$recipeId' })
  return <RecipeDetailView recipeId={recipeId} />
}

function RecipeEditPage() {
  const { recipeId } = useParams({ from: '/recipes/$recipeId/edit' })
  return <RecipeEditView recipeId={recipeId} />
}

const recipeDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/recipes/$recipeId',
  loader: recipeLoader,
  pendingComponent: RouteLoader,
  errorComponent: ({ error }) => <Alert color="red" m="md" title="Could not load the recipe">{String(error)}</Alert>,
  component: RecipeDetailPage,
})

const recipeEditRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/recipes/$recipeId/edit',
  loader: recipeLoader,
  pendingComponent: RouteLoader,
  errorComponent: ({ error }) => <Alert color="red" m="md" title="Could not load the recipe">{String(error)}</Alert>,
  component: RecipeEditPage,
})

// Cook mode needs only the recipe, not the tracks.
function RecipeCookPage() {
  const { recipeId } = useParams({ from: '/recipes/$recipeId/cook' })
  const { step } = useSearch({ from: '/recipes/$recipeId/cook' })
  return <CookModeView recipeId={recipeId} step={step} />
}

const recipeCookRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/recipes/$recipeId/cook',
  // The zero-based step lives in the URL, so reloading keeps the user on the same step.
  validateSearch: (search: Record<string, unknown>) => ({ step: toCookStep(search.step) }),
  loader: ({ context, params }) => context.queryClient.query(recipeQueryOptions(params.recipeId)),
  pendingComponent: RouteLoader,
  errorComponent: ({ error }) => <Alert color="red" m="md" title="Could not load the recipe">{String(error)}</Alert>,
  component: RecipeCookPage,
})

// The step lives in the URL, so reloading keeps the user on the same step.
// Steps read the tracks, recipes and this week's plan, so the loader fills all of them.
function OnboardingPage() {
  const { step } = useSearch({ from: '/onboarding' })
  return <OnboardingView step={step} />
}

const onboardingRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/onboarding',
  validateSearch: (search: Record<string, unknown>) => ({ step: toStep(search.step) }),
  loader: ({ context }) => Promise.all([
    context.queryClient.query(settingsQueryOptions),
    context.queryClient.query(tracksQueryOptions),
    context.queryClient.query(recipesQueryOptions),
    context.queryClient.query(scheduleQueryOptions(currentWeek())),
  ]),
  pendingComponent: RouteLoader,
  errorComponent: ({ error }) => <Alert color="red" m="md" title="Could not load setup">{String(error)}</Alert>,
  component: OnboardingPage,
})

// Sign in before onboarding finishes, or from the banner. A signed-in user goes straight on.
const signInRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/sign-in',
  validateSearch: (search: Record<string, unknown>): { next?: string } => {
    const next = toNextPath(search.next)
    return next ? { next } : {}
  },
  beforeLoad: async ({ context, search }) => {
    const status = await context.queryClient.query(syncStatusQueryOptions)
    if (status.mode === 'signed_in') throw redirect({ href: search.next ?? '/' })
  },
  component: function SignInPage() {
    const { next } = useSearch({ from: '/sign-in' })
    return <SignInView next={next} />
  },
})

// Joining needs an account, so a signed-out user goes to sign-in and comes back here afterwards.
const joinRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/join',
  beforeLoad: async ({ context }) => {
    const status = await context.queryClient.query(syncStatusQueryOptions)
    if (status.mode !== 'signed_in') throw redirect({ to: '/sign-in', search: { next: '/join' } })
  },
  component: JoinView,
})

// Placeholder until the Family page lands. The nav item only shows for signed-in accounts.
const familyRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/family',
  component: FamilyView,
})

const routeTree = rootRoute.addChildren([
  indexRoute,
  aboutRoute,
  recipeLinksRoute,
  recipeDetailRoute,
  recipeEditRoute,
  recipeCookRoute,
  onboardingRoute,
  familyRoute,
  signInRoute,
  joinRoute,
])

export const router = createRouter({ routeTree, context: { queryClient } })

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}
