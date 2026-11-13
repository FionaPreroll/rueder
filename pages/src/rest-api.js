// Shows the description of rueder's HTTP interface (openapi.json, written from
// the server's code) with Swagger UI, to read only: nothing is sent, as no
// server is behind this page; no other description is taken from the address;
// and the description goes to no validator elsewhere.

/* global SwaggerUIBundle */
window.addEventListener('load', () => {
	SwaggerUIBundle({
		url: 'openapi.json',
		dom_id: '#swagger-ui',
		deepLinking: true,
		docExpansion: 'none',
		defaultModelsExpandDepth: 0,
		filter: true,
		supportedSubmitMethods: [],
		tryItOutEnabled: false,
		queryConfigEnabled: false,
		validatorUrl: null
	});
});
