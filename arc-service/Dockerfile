FROM node:18-slim

WORKDIR /app

COPY package*.json ./
RUN npm install

COPY . .

EXPOSE 8084

CMD ["npx", "ts-node", "src/index.ts"]
