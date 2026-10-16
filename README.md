# C200 Studio

[![CI](https://github.com/FilipeJrd/c200-studio/actions/workflows/ci.yml/badge.svg)](https://github.com/FilipeJrd/c200-studio/actions/workflows/ci.yml)
[![AppImage](https://github.com/FilipeJrd/c200-studio/actions/workflows/appimage.yml/badge.svg)](https://github.com/FilipeJrd/c200-studio/actions/workflows/appimage.yml)

Aplicativo Linux independente para a **Anker PowerConf C200** (`291a:3369`),
com interface em Rust/egui, preview e perfis locais. Não é um produto oficial
da Anker e não depende do executável original em C.

## Executar

```sh
cargo run --release -p c200-app
```

Sem câmera, experimente a interface e os perfis com uma câmera simulada:

```sh
cargo run --release -p c200-app -- --demo
```

O modo de demonstração não acessa hardware e mantém a configuração em um
diretório temporário separado. Use `--config /caminho/settings.json` para
escolher outro arquivo, inclusive para persistir a demonstração.

## Recursos

- Interface Studio com prévia em destaque, seções Enquadramento/Imagem e
  preferências separadas para aparência, captura e diagnóstico.
- Joystick virtual de dois eixos: arraste e segure para mover, solte para parar.
  As setas do teclado ajustam a posição; Shift usa o menor passo do driver.
- Descoberta de C200s e reconexão automática, sem fixar `/dev/video0`.
- FOV de 65°, 78° e 95°, HDR, espelhamento e modo vertical proprietários.
- Brilho, contraste, saturação, matiz, gamma, nitidez, frequência da rede,
  balanço de branco, foco, exposição, pan, tilt e zoom via V4L2.
- Limites, opções de menu e disponibilidade obtidos do driver.
- Preview MJPEG/YUYV com seleção de resolução e FPS; preferência por 720p30.
- Perfis associados à câmera, reaplicados na abertura/reconexão enquanto o
  aplicativo estiver aberto; valores confirmados por leitura após alterações.
- Tema claro/escuro, HiDPI e operação por teclado.
- Diagnóstico JSON e modo de demonstração para desenvolvimento.

Em **Imagem → Opções técnicas**, o anti-flicker proprietário é um byte bruto `0..255`,
separado da frequência padrão V4L2. A semântica desses números ainda não foi
confirmada. O FOV bruto também está disponível para paridade com a CLI original.

Durante chamadas, use **Pausar prévia** para encerrar o streaming e deixar
a captura disponível para outro aplicativo. Ajustes continuam funcionando
quando o driver permite. Preview indisponível não bloqueia os demais controles.
O app não compartilha vídeo por câmera virtual e não captura áudio.

**Enquadramento** reúne campo de visão, joystick, zoom e espelhamento. O modo
vertical aparece quando o firmware o oferece. **Imagem** reúne brilho, foco,
cor e HDR; os controles manuais aparecem ao desativar o respectivo automático.
Exposição e outros ajustes ficam em grupos expansíveis. As capacidades e os
motivos de indisponibilidade podem ser consultados em **Preferências**.

O joystick usa os controles V4L2 reais de pan/tilt e seus limites e passos.
O retorno da alavanca ao centro apenas interrompe o envio de novas posições;
o enquadramento permanece onde você o deixou. Na câmera real, a prévia mostra
a imagem recebida do dispositivo, sem simular um recorte que só funcionaria
neste app. A demonstração simula o recorte para permitir experimentar o controle.

## Compilar no Linux

Instale Rust estável com Cargo, rustfmt e Clippy. O projeto usa egui/eframe
0.33, V4L2 direto e decodificação de imagens em Rust. Não requer Qt, GTK para
renderização, GStreamer nem `v4l2-ctl` em runtime. O build de V4L2 requer clang
e os headers do kernel; o sistema fornece OpenGL e as bibliotecas Wayland/X11.

Em CachyOS/Arch:

```sh
sudo pacman -S --needed rust clang pkgconf linux-api-headers libxkbcommon libxkbcommon-x11 wayland libx11 libxi libxrandr libxcursor libglvnd
cargo build --release --locked -p c200-app
./target/release/c200-studio
```

Em Ubuntu 22.04/24.04 (Rust estável instalado separadamente):

```sh
sudo apt-get install clang libclang-dev pkg-config libx11-dev libxi-dev libxrandr-dev libxcursor-dev libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libgl1-mesa-dev libegl1-mesa-dev libudev-dev libdbus-1-dev
cargo build --release --locked -p c200-app
```

## Configuração e perfis

Dados em `$XDG_CONFIG_HOME/c200-studio/settings.json`, ou
`~/.config/c200-studio/settings.json`. Nenhum arquivo de configuração é
gravado junto ao executável/AppImage. Arquivos inválidos ou de versão futura
são preservados e o app informa o problema em vez de sobrescrevê-los.

Escolha os ajustes, aguarde a confirmação e clique **Salvar perfil…** ou **Salvar**.
Salvar com o mesmo nome atualiza o perfil; selecionar um perfil o aplica.
Alterações posteriores só entram no perfil quando você salva novamente.
Trocar de perfil ou câmera com alterações oferece salvar, continuar sem salvar
ou cancelar. O status diferencia um ajuste pendente de confirmação de um ajuste
aplicado à câmera que ainda não foi salvo no perfil. Opções de renomear,
salvar como e excluir estão no menu junto ao seletor de perfis.
Um perfil não é aplicado a outra câmera com identidade diferente. Se a câmera
não tem serial USB, sua identidade depende da porta/hub físicos; movê-la pode
exigir criar ou selecionar um perfil novamente.

O aplicativo não instala serviço, não inicia na sessão automaticamente e não
promete persistência do firmware. Ao fechar a janela, encerra captura e
monitoramento. Sem perfil selecionado, a abertura apenas lê os controles.

## Diagnóstico e testes

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run -p c200-app -- --diagnose
cargo run -p c200-app -- --probe-preview
cargo run -p c200-app -- --demo --smoke-test --screenshot dist/demo.png
```

`--diagnose` apenas lê os controles. `--probe-preview` abre a captura,
negocia o modo de vídeo, recebe dez frames e libera os recursos, sem salvar
imagens ou alterar controles de imagem. Execute essas verificações também
na câmera real; testes simulados não comprovam compatibilidade de firmware.

O teste abaixo é opcional e exige a C200: altera pan, tilt e zoom em um passo,
confirma a leitura e restaura os valores originais. Não faz parte dos testes
executados normalmente ou no CI:

```sh
cargo test -p c200-app --locked framing_controls_round_trip_on_hardware -- --ignored --nocapture
```

Os sinais dos eixos e a exposição seguem a
[referência V4L2 do kernel](https://docs.kernel.org/userspace-api/media/v4l/ext-ctrls-camera.html).
O zoom é apresentado como múltiplo quando a C200 anuncia base 100; os valores
brutos e os limites permanecem acessíveis em Preferências e nas dicas dos controles.

Se o dispositivo não puder ser aberto, confira as ACLs da sessão e as regras
de acesso da sua distribuição. Execute o app como usuário normal. Se o preview
estiver ocupado, libere a captura no outro aplicativo ou mantenha apenas os
controles no C200 Studio. A tampa física da lente não é detectada pelo app.

## AppImage

```sh
cargo install cargo-about --version 0.9.2 --locked --features cli
bash scripts/appimage.sh
chmod +x dist/C200-Studio-x86_64.AppImage
./dist/C200-Studio-x86_64.AppImage
```

O script usa linuxdeploy e inclui os avisos de licença. Para compatibilidade
com Ubuntu 22.04 e sistemas mais novos, gere o pacote sobre Ubuntu 22.04,
como no workflow de CI. Um pacote compilado no CachyOS pode exigir uma glibc
mais recente e serve para teste local, não para garantir portabilidade.
Sem FUSE, execute o AppImage com `--appimage-extract-and-run`.

Para gerar diretamente sobre a base Ubuntu 22.04 com Docker:

```sh
bash scripts/portable-appimage.sh
```

## Estrutura

- `c200-core`: domínio, validação, aplicação ordenada de perfis e backend simulado.
- `c200-linux`: descoberta por sysfs, V4L2, Extension Unit e captura.
- `c200-app`: GUI, worker de controles, perfis e ciclo de vida do preview.

O hotplug é acompanhado a cada dois segundos por sysfs, sem daemon.
Workers executam captura e I/O da câmera fora da interface, com apenas o frame
mais recente pendente. Trocar o modo de vídeo preserva os ajustes da sessão.

O protocolo proprietário foi portado do projeto de Eran Sandler. Consulte
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) para origem, revisão e licença.

## Contribuir

Pull requests executam formatação, Clippy e testes em Ubuntu 22.04/24.04,
além de build e smoke test gráfico do AppImage. Os pacotes de desenvolvimento
e as capturas da demonstração ficam nos artefatos do workflow
[AppImage](https://github.com/FilipeJrd/c200-studio/actions/workflows/appimage.yml).
Veja [CONTRIBUTING.md](CONTRIBUTING.md) para o fluxo de desenvolvimento.
