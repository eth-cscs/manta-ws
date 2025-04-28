# MANTA-WS

#;TLDR

## Pipeline

The pipeline will trigger for all branches and for tag pushes.
If you want to push some changes to a development branch without triggering a potentially long-running build you can use:

```
git commit -m 'My commit message [skip ci]'
```
see https://docs.github.com/en/actions/managing-workflow-runs-and-deployments/managing-workflow-runs/skipping-workflow-runs

You can also manually trigger a build for a specific branch or tag. Navigate to Actions > Build and Push Docker Image to GHCR > Run workflow > Select appropriate ref

## Start API backend

```
cargo run
```

If need to connect to backends using a socks5 proxy then:

```
SOCKS5="socks5h://127.0.0.1:1080" cargo run
```

## Start frontend

```
cd assets
npm run dev
```

Open a web browser and go to http://localhost:5173/ or go to the url shown by the npm dev web server after running `npm run dev` command

## Introduction

Test project to explore a web client to consume Alps services

## Test

### Cmd client

Use websocat client for testing. Curl only accepts websocket as EXPERIMENTAL hence it needs to be enabled during build process, you can check whether curl supports websockets if protocol `ws` is listed in `curl --version`.

As an alternative, use https://github.com/vi/websocat

#### Installation

#### Run server (local machine for testing)

```
SOCKS5="socks5h://127.0.0.1:1080" cargo run
```

#### Run web server (local machine through npm for testing)

```
cd assets
npm run dev
```

#### Test: Cli client (websocket)

I could not get curl to work with websockets (last time I checked was2023) because the curl version installed on my laptop did not have websocket features. Instead I found websocket which is a cli to interact with websockets

Install websocat

```
wget https://github.com/vi/websocat/releases/download/v1.11.0/websocat.x86_64-unknown-linux-musl
chmod +x websocat.x86_64-unknown-linux-musl
mv websocat.x86_64-unknown-linux-musl ~/.local/bin/websocat
```

Example

```
websocat -E ws://localhost:3000/console/x1000c4s0b0n0
websocat -E ws://localhost:3000/cfssession/batcher-4176d230-a813-48cd-9328-b1ba7da68d99/logs
```

#### Test: Cli client (http)

```
$ curl -H "Authorization: Bearer $TOKEN" http://localhost:3000/cfs/health
{"db_status":"ok","kafka_status":"ok"}
```

### Test: Web client

 - Open a browser
 - Go to http://localhost:5173/console/x1000c4s0b0n0 to start xterm.js
 - Go to http://localhost:5173/cfssession/batcher-4176d230-a813-48cd-9328-b1ba7da68d99/logs

### Deploy on remove server

Build artifact

```
$ cargo build --release --target x86_64-unknown-linux-musl
```

Run installation script

```
$ scripts/install.sh
```

## Development

#### Setup web client development environment

Vuejs development environment (ref https://vuejs.org/guide/quick-start.html#creating-a-vue-application)

```
❯ npm init vue@latest
Need to install the following packages:
  create-vue@3.6.1
Ok to proceed? (y)

Vue.js - The Progressive JavaScript Framework

✔ Project name: … 
✔ Add TypeScript? … No / Yes
✔ Add JSX Support? … No / Yes
✔ Add Vue Router for Single Page Application development? … No / Yes
✔ Add Pinia for state management? … No / Yes
✔ Add Vitest for Unit Testing? … No / Yes
✔ Add an End-to-End Testing Solution? › No
✔ Add ESLint for code quality? … No / Yes
✔ Add Prettier for code formatting? … No / Yes

Scaffolding project in /home/msopena/polybox/Documents/tests/rust//assets/manta-ws...

Done. Now run:

  cd 
  npm install
  npm run format
  npm run dev


/assets on  master [!?] via  v18.16.0 took 59s
❯ cd 

/assets/manta-ws on  master [!?] via  v18.16.0
❯ npm install
npm WARN deprecated sourcemap-codec@1.4.8: Please use @jridgewell/sourcemap-codec instead

added 146 packages, and audited 147 packages in 18s

32 packages are looking for funding
  run `npm fund` for details

found 0 vulnerabilities

/assets/manta-ws on  master [!?] via  v18.16.0 took 18s
❯ ls
index.html  node_modules  package.json  package-lock.json  public  README.md  src  vite.config.js

/assets/manta-ws on  master [!?] via  v18.16.0
❯ ls src/
App.vue  assets  components  main.js  router  stores  views

/assets/manta-ws on  master [!?] via  v18.16.0
❯ npm run format

> @0.0.0 format
> prettier --write src/

src/App.vue 105ms
src/assets/base.css 17ms
src/assets/main.css 6ms
src/components/HelloWorld.vue 24ms
src/components/icons/IconCommunity.vue 4ms
src/components/icons/IconDocumentation.vue 4ms
src/components/icons/IconEcosystem.vue 2ms
src/components/icons/IconSupport.vue 2ms
src/components/icons/IconTooling.vue 5ms
src/components/TheWelcome.vue 21ms
src/components/WelcomeItem.vue 15ms
src/main.js 8ms
src/router/index.js 13ms
src/stores/counter.js 8ms
src/views/AboutView.vue 5ms
src/views/HomeView.vue 4ms

/assets/manta-ws on  master [!?] via  v18.16.0
❯ npm run dev

> @0.0.0 dev
> vite


  VITE v4.3.3  ready in 338 ms

  ➜  Local:   http://localhost:5173/
  ➜  Network: use --host to expose
  ➜  press h to show help
```

Additionally add xterm and  xterm-addon-fit (ref https://xtermjs.org/docs/guides/download/ and https://www.npmjs.com/package/xterm-addon-fit)

```
npm install --save xterm
npm install --save xterm-addon-fit
npm install --save xterm-addon-attach
```

Install fonts

```
npm install @mdi/font -D
npm i --save @fortawesome/fontawesome-svg-core
npm install @mdi/svg @mdi/util
npm install @mdi/js -D
```
