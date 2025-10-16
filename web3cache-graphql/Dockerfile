FROM --platform=linux/amd64 node:16
WORKDIR /app
COPY package.json package.json
RUN yarn install
COPY src/ src/
COPY config/ config/
EXPOSE 4000
CMD ["yarn" ,"start"]