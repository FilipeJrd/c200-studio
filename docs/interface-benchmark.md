# Proposta de interface do C200 Studio

A direção Studio foi implementada na versão 0.2.0, incluindo joystick, zoom,
duas seções de ajustes e o fluxo de salvar perfis. A alternativa Compacto
permanece como proposta. Este documento preserva a análise que orientou o desenho;
consulte README.md e VALIDATION.md para o comportamento e a validação da versão.

Pesquisa em 16/10/2026. Benchmark qualitativo de documentação e capturas públicas dos fabricantes, comparado com a tela e o código do C200 Studio. Os aplicativos oficiais não foram instalados nem submetidos a testes de uso. As capturas publicadas podem representar versões e dispositivos diferentes dos atuais.

## Recomendação

Adotar a proposta **Studio**: prévia dominante à esquerda e um inspetor enxuto à direita, com duas seções estáveis: **Enquadramento** e **Imagem**. Ajustes adicionais aparecem por expansão. Usar a simplicidade do Logi Tune como referência de prioridade e os grupos do AnkerWork/Camera Hub como referência de organização. Manter Rust/egui para esta etapa: o problema observado é principalmente hierarquia, composição e comportamento dos controles. Reavaliar o framework se uma prova visual em egui demonstrar limitações concretas.

O modo **Compacto** é uma alternativa para ajustes rápidos, com prévia acima dos controles. Pode entrar depois; não é necessário implementar duas experiências para entregar o primeiro redesenho.

## Benchmark

| Referência | Evidência observada | Aplicação ao C200 Studio | Cuidado |
| --- | --- | --- | --- |
| [AnkerWork — Software Download Center](https://us.ankerwork.com/pages/download-software/) | As capturas mostram grupos de controles, alternância automático/manual e indicações visuais de brilho e temperatura. | Agrupar por intenção e explicar os extremos dos sliders; manter a relação entre automático e manual junto ao controle. | As imagens incluem recursos de outros dispositivos, como iluminação. Não tratar a captura publicada como uma tela específica da C200 nem importar esses recursos. |
| [Logi Tune — manual oficial da Brio 500, p. 8 / 32](https://www.logitech.com/assets/66219/2/brio-500.pdf) | Tela compacta com prévia, seleção direta do campo de visão e entrada separada para ajustes de imagem. | Dar acesso imediato ao enquadramento e deixar ajustes menos frequentes em um segundo nível. | O manual é de 2022. É uma referência documentada de layout, não verificação da versão mais recente. Os ângulos da Brio não são os da C200. |
| [Elgato Camera Hub — Settings Overview](https://help.elgato.com/hc/en-us/articles/4405055113357-Elgato-Facecam-Camera-Hub-Settings-Overview) | Seções de dispositivo, zoom, imagem, exposição, balanço de branco e processamento; dependências explícitas dos modos automáticos. | Criar grupos coerentes e revelar o ajuste manual perto de seu modo automático. | O painel completo é denso para nosso uso cotidiano. Salvar na memória da câmera é um recurso daquele produto; não presumir suporte equivalente na C200. |
| [Camo Studio — Navigating Camo’s interface](https://camo.com/support/camo/how-to-use-camo) | Prévia central dominante, estados de conexão distintos e um painel rápido para ações frequentes. Guia atualizado em julho de 2026. | Dar peso à imagem, separar falha de conexão de prévia pausada e reservar uma experiência compacta para uso rápido. | A interface completa tem dois painéis laterais e recursos de produção. Reproduzir toda a densidade seria inadequado ao escopo atual. |

As recomendações de usabilidade abaixo são inferências de design a partir dessas referências e da inspeção do MVP, não resultados medidos com usuários.

## Problemas observados no MVP

1. O cabeçalho usa duas faixas e mantém marca, slogan, tema, câmera, reconexão e perfis em evidência ao mesmo tempo.
2. As quatro abas do inspetor quebram em duas linhas. O espaço reservado à navegação reduz o espaço útil para ajustes.
3. Cada controle recebe título, editor, texto auxiliar e separador semelhantes. Há pouca diferença entre uma escolha frequente e um ajuste técnico.
4. Selecionar um perfil já o aplica, mas existe outro botão “Aplicar”. Isso torna a ação redundante e deixa a relação entre aplicar e salvar menos clara.
5. “Liberar câmera” exige conhecer a disputa de captura entre aplicativos. A ação fica mais compreensível como “Pausar prévia”, acompanhada de sua consequência.
6. Controles indisponíveis e valores brutos aparecem junto ao fluxo comum, embora sejam úteis principalmente para compatibilidade e diagnóstico.

## Estrutura proposta

### Cabeçalho

- Nome C200 Studio em tamanho discreto.
- Nome da câmera e estado de conexão. Exibir seletor de dispositivos somente quando houver alternativas ou quando for necessário escolher uma câmera.
- Preferências para aparência, formato da prévia, informações e diagnóstico.
- Reconexão contextual nos estados de falha; não precisa ocupar a tela durante operação normal.

### Área de imagem

- Prévia com proporção correta, sem deformação ou recorte involuntário.
- Perfil salvo próximo da imagem; selecionar aplica, salvar atualiza o perfil explicitamente.
- Ação “Salvar” habilitada quando os valores confirmados diferirem do perfil. Primeiro uso oferece “Salvar perfil…”. Nomear, salvar como, renomear e excluir ficam no menu de perfis.
- Resolução e FPS descritos como **formato da prévia deste app**. Não sugerir que essa escolha configura a resolução do Meet, Zoom ou OBS.
- Ação “Pausar prévia”: fecha somente a captura do C200 Studio. Não significa desligar a câmera globalmente nem interromper outra aplicação.
- Evitar um botão de captura de foto ou gravação nesta etapa, pois esses recursos não fazem parte do backend atual.

### Inspetor

| Grupo | Controles e comportamento |
| --- | --- |
| Enquadramento | 65° Fechado, 78° Equilibrado, 95° Amplo; joystick de deslocamento horizontal/vertical, zoom e espelhamento. Todos ficam juntos no primeiro nível. |
| Imagem | Brilho, foco automático, cor automática e HDR, conforme disponibilidade real do dispositivo. Foco e cor manuais aparecem ao desligar seu automático. |
| Mais ajustes de imagem | Exposição automática/manual, contraste, saturação, matiz, gamma e nitidez. Manter os controles previstos pelo backend e respeitar suas dependências. |
| Opções técnicas | Frequência da rede elétrica, anti-flicker proprietário com valor bruto e explicação, modo vertical quando suportado e detalhes de compatibilidade. Não inventar rótulos em Hz para valores proprietários ainda não decodificados. |

Preservar os 21 controles conhecidos, acessíveis por expansão e opções técnicas, sem exigir que todos apareçam na tela inicial. Nunca esconder silenciosamente falhas de leitura: capacidades ausentes aparecem nos detalhes de compatibilidade; falhas de operação devem gerar feedback contextual.

### Joystick de enquadramento

Refinamento solicitado após a primeira proposta: manter os deslocamentos visíveis junto do campo de visão, com um joystick virtual de dois eixos e zoom ao lado.

- Arrastar e segurar indica direção e intensidade. Movimento diagonal atua nos dois eixos, com velocidade total limitada para não acelerar na diagonal.
- Soltar faz a alavanca voltar ao centro e interrompe o movimento; a imagem conserva a posição escolhida. Esse retorno não recentraliza a câmera.
- As setas do teclado fazem passos pequenos; Shift + seta oferece passos ainda menores.
- O estado persistente é a posição horizontal/vertical e o zoom; a posição temporária da alavanca não entra no perfil.
- Cancelamento do gesto, perda de foco e troca de seção ou janela precisam interromper o movimento. A implementação Rust deve fazer o mesmo ao desconectar o dispositivo.
- Usar `pan_absolute`, `tilt_absolute` e `zoom_absolute` existentes, convertendo direção em incrementos limitados pelos valores mínimo, máximo e passo anunciados pelo driver. Não criar uma função de recorte somente na prévia que pareça alterar a imagem dos outros aplicativos.
- A implementação deve conferir quais combinações de FOV/zoom permitem deslocamento e informar limites. O protótipo apenas simula essa relação por recorte e mantém a imagem dentro da área disponível; não é prova do comportamento do firmware.
- Não adicionar rastreamento automático ou movimento motorizado à proposta. O controle representa os deslocamentos já previstos no app.

A primeira proposta colocava zoom e deslocamentos no segundo nível; esta revisão os promove para a seção principal de Enquadramento. As duas abas cabem em uma única linha.

### Aplicação e perfis

1. O usuário muda um controle.
2. A interface indica aplicação pendente de maneira discreta, sem mover o layout.
3. O backend confirma pelo readback; só então a interface mostra “Aplicado à câmera · perfil não salvo”.
4. Salvar persiste o perfil local e mostra “Perfil Trabalho · salvo”. Não afirma que o firmware guardou o perfil.
5. Uma falha mantém o valor confirmado anterior e explica o problema próximo ao controle.
6. Trocar de perfil com alterações não salvas oferece salvar, descartar ou cancelar. Esse tratamento precisa ser implementado; o protótipo ilustra apenas a seleção e o estado de edição.

Os nomes “Trabalho” e “Luz natural” no protótipo representam perfis de exemplo. Não implicam presets universais de iluminação nem detecção automática de ambiente.

### Estados

| Estado | Mensagem e ação |
| --- | --- |
| Sem câmera | “Conecte sua PowerConf C200.” Reconhecimento automático e acesso ao diagnóstico se necessário. |
| Conectando | Indicação discreta de conexão; controles preservam posição e aguardam leitura. |
| Prévia ativa | Imagem e ação “Pausar prévia”. |
| Prévia pausada | “A captura deste app está liberada para usar a câmera em uma chamada.” Ação “Retomar prévia”. |
| Captura indisponível | “Não foi possível iniciar a prévia.” Sugerir outra captura apenas quando houver evidência; oferecer tentativa e detalhes. Manter controles disponíveis quando o driver permitir. |
| Desconectada durante uso | Preservar perfil e informar desconexão; desabilitar escrita até reconectar. |

Não afirmar que outro aplicativo usa a câmera apenas porque a captura falhou. Não identificar aplicativos sem uma fonte real dessa informação.

## Direção visual

- Paleta neutra com um único destaque menta; tons claros e escuros seguem o sistema.
- Grade de espaçamento de 8 px. Preferir controles com altura de 36–40 px em desktop e áreas de interação maiores quando necessário.
- Texto de controle em 14 px, títulos de grupo em 15–16 px e título da área em 18 px; apoio em 12 px com contraste suficiente.
- Inspetor com aproximadamente 308–340 px. Janela de referência próxima de 1100 × 760; adaptar abaixo disso sem quebrar a navegação em múltiplas linhas.
- Poucas divisórias, agrupamento por espaço, cantos discretos e estados de foco visíveis.
- Evitar animações contínuas, saltos de layout, pop-ups de sucesso para cada slider e excesso de ícones sem rótulo.
- Transições curtas opcionais, respeitando redução de movimento. Prioridade para resposta fluida e texto legível em escalas de tela de 100%, 150% e 200%.

## Implementação sugerida

1. Criar tokens de estilo e componentes egui: cabeçalho, seletor segmentado, linha de ajuste, alternância, seção expansível e indicador de estado. Começar por uma tela fiel à proposta Studio.
2. Substituir as quatro abas atuais pelas duas seções Enquadramento e Imagem, com divulgação progressiva dentro de Imagem. Implementar o joystick com captura do ponteiro, cancelamento do gesto e acesso por teclado. Renderizar controles conforme capacidades e dependências; manter acesso aos controles técnicos.
3. Separar estados de captura, conexão, aplicação pendente e perfil alterado. Resolver o fluxo de perfil sem duplicar “Aplicar”.
4. Preservar threads de captura/comandos existentes. Evitar operações de dispositivo no render e manter o slider responsivo enquanto a confirmação vem do backend.
5. Validar com a C200 real: gravação e readback, pausa que libera captura, reconexão, foco/cor automáticos, ausência de modo vertical e alterações externas.
6. Gerar novo AppImage somente após validar o redesenho. O executável e o AppImage atuais não foram modificados por este estudo.

Não é necessário migrar para Tauri para experimentar essa proposta. O protótipo em conversa representa aparência e fluxo; ele não demonstra paridade de renderização entre HTML e egui. Uma implementação de referência em egui deve ser julgada antes de decidir por uma migração.

## Critérios de aceite

- A tela inicial permite reconhecer a câmera, ver a imagem, escolher o enquadramento e encontrar o perfil sem abrir preferências.
- FOV, zoom, joystick e pausa ficam visíveis no enquadramento inicial; brilho fica na seção Imagem, acessível com uma seleção. A seleção de perfil permanece disponível sem abrir outra tela.
- Soltar o joystick, cancelar o gesto, trocar de seção ou perder foco interrompe o movimento sem perder a posição escolhida.
- O joystick aceita diagonais e setas do teclado, e não permite ultrapassar os limites da câmera.
- Ajustes manuais deixam clara sua dependência do modo automático.
- A confirmação de aplicação usa o valor lido de volta da câmera; o valor não salta para o padrão durante a leitura.
- Um controle indisponível não parece um botão funcional. A razão permanece acessível em compatibilidade.
- Navegação por teclado e foco são visíveis; escala de 150% não corta controles ou textos.
- O fluxo não perde alterações de perfil silenciosamente.
- Janela menor mantém ações essenciais acessíveis. O modo Compacto pode ser implementado depois da janela Studio.

## Limites do protótipo

As duas propostas usam uma ilustração de escritório e respostas visuais aproximadas para campo de visão, zoom, posição, brilho, contraste, saturação e espelhamento. Não acessam a câmera. A escala de zoom e a posição normalizada são ilustrativas; a versão instalada deve usar os intervalos reais anunciados pelo dispositivo. Sliders em porcentagem são uma convenção de apresentação: a implementação precisa converter o intervalo e o passo anunciados pelo driver, preservar o valor bruto em detalhes e usar unidades físicas somente quando conhecidas. O protótipo cobre os fluxos principais, não todos os controles e diálogos do produto.
