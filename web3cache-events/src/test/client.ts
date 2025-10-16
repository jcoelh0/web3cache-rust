// testing purposes
import { io, Socket } from "socket.io-client";

const token: string = "eyJhbGciOiJSUzUxMiIsInR5cCI6IkpXVCJ9.eyJhZGRyZXNzIjoiMHgwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwIn0.h6FTrroRFELKpCkl3tbX-NbP4Dxx5Ld5EvqfdSjfWnAmFLnt4Q6R3VK7GxzN3TKoHLH0QdMXJeCZjfDB9yvZ9mLDlIhawwKLEqUx6wyeidNNmGXTZejXENVp_IKyyJrxX5VKCd91d73M0gnlwbxLGZs30ngCgurS5VEbqtMCjpQuWFLCdVAeVkY4rvyQAD_T8zS6C6mMwz91n4OTEqNEAgZ9IOPBDZ6v6Ejc-54BETj2_yQoHUcsKa3m-ZKw9J3dRjL8hn7Nlyenvzf43hoJduQ9aENNfsMPah3L_haXHaTe4HvoqwWNl0QH-hDlIgqy7FNQVKqOzKjrNBohKjNINiLorqziTxPX6U4w2IVtD9CgrtN9ldQ-gqqsyIlu5z5Tcj1p5EMYLQmSZTxnORsDyk2zOE5g_NUb4fdUEermFqkUGYmCnir0F6l2_5NB6c01gbMrRipi_IZLBJbnmO9Q43Pd3GYhv-Fg46j9NqL1r8VzDg1OMk9vUzHj58zKNd6EIAMeJYjoaADiS6ZJ1uhWHHgAJNQDZMb1FEF39_6r8ddvT4qLGNcYwBkmkQr6nRNgvKK1mY0YlI6-EUDjkfqOo3TuarQI01K3yQ2ahYJpLhcCXBbbDLXCnF0LHF65t-JWOUfLzjvLFqMhQNA6V8lA5qZTzecf30T8CtO---mowMA"


class Client {
    private socket: Socket

    constructor() {
        this.socket = io('ws://localhost:3000', {
            withCredentials: true,
            extraHeaders: {
                cookie: `x-auth-token=${token}`
            }
        });
        this.socket.on("connect", () => {
            console.log("connection!!");
            this.socket.emit("login");
            console.log("after login");
        })
        this.socket.on("transaction", (transaction: any) => {
            console.log("transaction", transaction);
        })

    }
}

const client = new Client()