FROM node:16
WORKDIR /app
COPY package.json package.json
RUN npm install
COPY src/ src/
COPY config/ config/
RUN npm i ts-node -g
EXPOSE 3000
EXPOSE 3003
CMD ["npm" ,"run", "dev"]