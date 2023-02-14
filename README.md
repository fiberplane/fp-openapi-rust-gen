# Fiberplane OpenAPI Rust Client Generator

`fp-openapi-rust-gen` is a tool to generate Rust client code from OpenAPI 3.0 specifications.
It was created as an in-house replacement for the [OpenAPI Tools openapi-generator][0] with a feature set
specifically tailored (but not exclusive) to the needs of Fiberplane.

The main differences are:

* The generated code references pre-defined models from the `fiberplane` crates instead of generating its own models.
  This allows us to use the same models in the generated code as in the rest of our codebases, including our backend, frontend and CLI.
* Support has been added for common Fiberplane-specific types such as `Base64Uuid`.
* There is first class support for `HashMap` and `time` data types out of the box, including in query parameters.

## Getting Help

Please see [COMMUNITY.md][fp-com] for ways to reach out to us.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## Code of Conduct

See [CODE_OF_CONDUCT.md][fp-coc].

## License

All code within the `fp-openapi-rust-gen` repository is distributed under the terms of
both the MIT license and the Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE.txt) and [LICENSE-MIT](LICENSE-MIT.txt).

[0]: https://github.com/OpenAPITools/openapi-generator
[1]: https://github.com/fiberplane/fp-openapi-rust-gen/issues
[2]: (https://github.com/fiberplane/fp-openapi-rust-gen/discussions)

[fp-com]: https://github.com/fiberplane/fiberplane/blob/main/COMMUNITY.md
[fp-coc]: https://github.com/fiberplane/fiberplane/blob/main/CODE_OF_CONDUCT.md
