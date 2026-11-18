// pg 8.16.3 or later is recommended
import { Client } from "pg";

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

		try {
			// Connect to the database
			await sql.connect();

			// Sample query
			const results = await sql.query(`SELECT * FROM pg_tables`);

			// Return result rows as JSON
			return Response.json(results.rows);
		} catch (e) {
			console.error(e);
			return Response.json(
				{ error: e instanceof Error ? e.message : e },
				{ status: 500 },
			);
		}
	},
} satisfies ExportedHandler<Env>;