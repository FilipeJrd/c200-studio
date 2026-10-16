# Validação da versão 0.2.0

## Desenvolvimento

- Rust 1.99.0 do sistema, CachyOS x86_64, KDE/Wayland.
- `cargo fmt --all -- --check` e Clippy com `-D warnings`.
- 24 testes automatizados de protocolo, limites e menus, dependências de modos
  automáticos, perfis, confirmação após escrita, liberação da captura e preview.
- Joystick: diagonais com velocidade limitada, acúmulo de movimentos menores
  que o passo do driver, parada ao soltar/perder foco e passos por teclado.
- Escritas contínuas limitadas a lotes a cada 80 ms, com consolidação no worker
  e tickets para impedir que confirmações antigas removam edições mais recentes.
- Perfis: decisão antes de trocar com alterações, espera por readback, rollback
  da alteração em memória quando a gravação falha e leituras automáticas que
  não marcam o perfil como editado.
- Interface Studio de demonstração renderizada e inspecionada nos temas claro
  e escuro, incluindo joystick, zoom e organização dos perfis.
- Janela com câmera real aberta e encerrada pelo smoke test, usando configuração
  temporária e sem aplicar perfis.

## Hardware

Anker PowerConf C200, USB `291a:3369`:

- Descoberta como `/dev/video0`, excluindo o nó de metadados.
- Leitura de 20 dos 21 controles do catálogo, incluindo FOV, HDR, espelhamento
  horizontal e anti-flicker proprietário.
- `vertical_screen` não é oferecido pelo firmware desta unidade e aparece
  indisponível. Isso não bloqueia os outros controles.
- Preview MJPEG 1280×720, 30 FPS: dez frames decodificados e captura liberada.
- Nove combinações de formato/resolução/FPS enumeradas, até 2560×1440 em MJPEG.

Em 16/10/2026 foi executado também o teste opt-in de enquadramento na C200:

- Pan e tilt: passo de 3.600, limites de -36.000 a 36.000; escrita de um passo,
  leitura de confirmação e restauração de ambos para 0 confirmadas.
- Zoom: limites de 100 a 400; escrita de um passo, confirmação e restauração
  do valor original 295 confirmadas.

Os demais testes de escrita usam transportes simulados. Não foi feita uma
varredura de alterações reais dos 21 controles nem uma medição visual do recorte
em todos os níveis de FOV/zoom. A confirmação de pan/tilt/zoom não prova a
compatibilidade de todos os firmwares C200.

## Distribuição

O Dockerfile prepara o AppImage sobre Ubuntu 22.04, executa os testes e abre
o pacote em X11 virtual com renderização por software. O arquivo
`dist/ubuntu22-demo.png` é produzido por essa verificação. O pacote também
deve passar pelo smoke test no desktop de destino antes da distribuição.

O AppImage 0.2.0 passou no teste gráfico Ubuntu 22.04, na abertura normal com
FUSE no CachyOS e na captura real de dez frames da C200. O checksum SHA-256 foi
conferido. O teste virtual usa X11 explícito e timeout de 25 segundos; executar
`xvfb-run` como PID 1 do contêiner pode impedir seu protocolo de inicialização.

O CI repete validação e empacotamento na mesma base. Ubuntu 24.04 e outros
desktops/distribuições ainda precisam de validação dedicada; compatibilidade
não é inferida apenas da geração do AppImage.
