---
layout: home

hero:
  name: Drahtseilakt
  text: Ballance rollt wieder
  tagline: Ein natives Spiel auf Basis von OpenBallance, einer originalgetreuen Neuimplementierung des Klassikers von 2004. Die originalen Spielskripte laufen auf einer neuen, Virtools-kompatiblen Laufzeitumgebung mit einem Port der spieleigenen Physik-Engine, mit den Daten Ihrer eigenen Kopie des Spiels.
  image:
    src: /screenshots/level01.jpg
    alt: Die Holzkugel rollt durch Level 1
  actions:
    - theme: brand
      text: Fahrplan
      link: /de/drahtseilakt/roadmap
    - theme: alt
      text: Entscheidungen
      link: /de/drahtseilakt/decisions

features:
  - title: Originalgetreu
    details: Die Logik von Ballance steckt in Virtools-Verhaltensgraphen, und die Engine führt genau diese Originalgraphen aus. Die Building Blocks, die sie aufrufen, sind einzeln aus den Original-DLLs portiert, mit allen Eigenheiten.
  - title: Die originale Physik
    details: Das Gefühl der Kugel kommt von der Physik-Engine Ipion in physics_RT.dll. Ihr Kern, die Kollisionserkennung, Reibung und Stöße sind aus der Disassemblierung portiert, Konstante für Konstante.
  - title: Ihre Daten
    details: Level, Texturen, Klänge und Musik werden direkt von Ihrem Disc-Image, der CD selbst oder einer installierten Kopie gelesen. Keine Assets im Paket.
  - title: Dokumentation auf Deutsch
    details: Fahrplan und Entscheidungen des Forks gibt es schon auf Deutsch. Benutzerhandbuch und Interna sind bisher nur auf Englisch, die Übersetzung steht im Fahrplan.
---
