// pg 8.16.3 or later is recommended
import { Client } from "pg";
import Cursor from "pg-cursor";

export interface Env {
    // If you set another name in the Wrangler config file as the value for 'binding',
    // replace "HYPERDRIVE" with the variable name you defined.
    HYPERDRIVE: Hyperdrive;
}

export default {
    async fetch(request, env, ctx): Promise<Response> {
        // Create a new client on each request. Hyperdrive maintains the underlying
        // database connection pool, so creating a new client is fast.
        const sql = new Client({
            connectionString: env.HYPERDRIVE.connectionString,
        });

        const makeErrorLogger = (event: string) => (error) => {
            console.error(`${event} error`, error);
        };

        sql.connection.on("errorMessage", makeErrorLogger("onBackendError"));
        sql.on("error", makeErrorLogger("onClientError"));
        sql.on("end", () => console.info("connection ended"));

        // Connect to the database
        await sql.connect();

        await sql.query("BEGIN");

        const cursor = sql.query(
            new Cursor("SELECT generate_series(1, 10) AS n"),
        );
        const rows = await cursor.read(2);
        console.log("Rows:", rows);

        await Promise.race([cursor.close(), timeout(10_000)]);

        console.log("cursor closed");
        await sql.query("COMMIT");

        // Return result rows as JSON
        return Response.json({ ok: true });
    },
} satisfies ExportedHandler<Env>;

const timeout = (ms: number) =>
    new Promise((_, reject) =>
        setTimeout(() => reject(new Error("Portal close timed out")), ms),
    );
