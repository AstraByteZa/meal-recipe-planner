# Deploying the sync server on Dokploy

The sync server is an Axum app with a Postgres database. The desktop app talks to it over HTTPS.
Deploy it as two Dokploy services in one project: a managed Postgres database, and an application
built from `sync-server/Dockerfile`.

## Before you start

- **One replica only.** Live notifications use an in-memory hub, so a second replica would not
  hear about changes made through the first. Leave the replica count at 1.
- **HTTPS is required.** The mobile apps refuse plain HTTP for sync traffic in practice, and
  Dokploy's Traefik can issue the certificate for you.
- **Back up Postgres.** Enable scheduled backups on the database service. Migrations run at
  startup, so take a backup before deploying a release that adds one.

Generate the JWT secret once and keep it. Changing it signs everyone out:

```sh
openssl rand -base64 48
```

## 1. Create the database

1. In your Dokploy project, add a **PostgreSQL** service. Use Postgres 17.
2. Set a strong password and the database name `planner_sync`.
3. Note the **internal host name** Dokploy shows for the service. Services in the same project reach
   each other by that name, not by a public address. The examples below use `planner-db`.

## 2. Create the application

1. Add an **Application** service and connect it to the Git repository and branch you deploy from.
2. Set **Build type** to **Dockerfile**.
3. Set **Dockerfile path** to `sync-server/Dockerfile`.
4. Set **Docker context path** to `.` (the repository root). The workspace needs every member's
   manifest, so the context cannot be `sync-server/`.

## 3. Set the environment

```env
DATABASE_URL=postgres://planner:<password>@planner-db:5432/planner_sync
JWT_SECRET=<output of openssl rand -base64 48>
BIND_ADDR=0.0.0.0:8080
REGISTRATION_OPEN=true
RUST_LOG=sync_server=info
```

| Variable | Required | Meaning |
| --- | --- | --- |
| `DATABASE_URL` | yes | Postgres connection string. Use the internal host name. |
| `JWT_SECRET` | yes | Signs login tokens. At least 32 bytes. |
| `BIND_ADDR` | no | Listen address. Defaults to `0.0.0.0:8080`. |
| `REGISTRATION_OPEN` | no | `false` refuses new accounts. Defaults to open. |
| `RUST_LOG` | no | Log filter, for example `sync_server=debug`. |

## 4. Add the domain

1. Under the application's **Domains**, add a host name such as `sync.example.com`.
2. Set the container port to **8080**.
3. Turn on HTTPS with Let's Encrypt.

## 5. Deploy and check

Deploy the application, then check the health endpoint:

```sh
curl -fsS https://sync.example.com/health
# ok
```

The image's own health check calls the same endpoint on the container's port 8080.

## 6. Create accounts, then close registration

Register the household's accounts from the app (**Sync settings**). Then set
`REGISTRATION_OPEN=false` and redeploy. Existing accounts can still sign in, and nobody else can
create one. Joining a family needs only an invite code, so closed registration does not block it.

## 7. Connect the app

In the desktop app, open **Sync settings** and enter `https://sync.example.com` as the server.
Sign in, then create a family or join one with its invite code.

## Checking a deployment locally

To run the same image on your machine with a local Postgres:

```sh
JWT_SECRET=$(openssl rand -base64 48) docker compose -f sync-server/docker-compose.yml up --build
curl -fsS http://localhost:8080/health
```

## Troubleshooting

- **Container exits right away:** check the logs. A missing `DATABASE_URL` or a `JWT_SECRET` shorter
  than 32 bytes stops startup with a message that names the variable.
- **Migration errors at startup:** the database is untouched by a failed migration, since each
  migration runs in a transaction. Fix the cause and redeploy.
- **Clients see no live updates but do get changes on sync:** the stream may be going through a
  proxy that buffers responses. The server sends `X-Accel-Buffering: no`, which Traefik honours.
  Check that nothing else in front of Traefik buffers.
