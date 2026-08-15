FROM node:22-bookworm-slim
WORKDIR /workspace
COPY package.json package-lock.json ./
RUN npm ci
COPY tsconfig.json vitest.config.ts ./
COPY app ./app
CMD ["npm", "run", "preflight"]
