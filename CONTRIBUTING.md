# Contribuir

Crie uma branch a partir de `main` e abra um pull request com uma descrição
do problema, da mudança e dos testes realizados. Consulte o [README](README.md)
para instalar as dependências nativas. O `rust-toolchain.toml` fixa a versão
do Rust e os componentes usados no desenvolvimento e no CI.

Antes de enviar:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Para experimentar a interface sem acessar uma câmera:

```sh
cargo run -p c200-app -- --demo
```

Os pull requests executam esses checks em Ubuntu 22.04 e 24.04, além de gerar
um AppImage sobre Ubuntu 22.04 e abrir a interface em X11 virtual. O workflow
AppImage disponibiliza o pacote, seu checksum e capturas da demonstração na
seção **Artifacts** da execução por 14 dias. Esses artefatos são builds de
desenvolvimento do commit verificado.

Testes que escrevem na câmera real são opt-in e ficam fora do CI. Consulte
o README antes de executá-los. A demonstração e os testes automatizados não
comprovam compatibilidade com todos os firmwares; registre verificações reais
em mudanças de protocolo ou captura. Não inclua configurações pessoais,
números de série, diagnósticos particulares ou imagens da câmera nos commits.

Os workflows usam `pull_request`, permissões somente de leitura e Actions
fixadas por commit. Mantenha essas propriedades ao modificá-los. Caches só
são gravados por pushes na `main`. O pacote usa a mesma versão do Rust
definida no projeto; ao alterá-la, atualize também o Dockerfile.
