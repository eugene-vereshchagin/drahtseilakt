# Warum Ballance ein zweites Leben brauchte

Ballance ist nicht verblasst, weil es alt wurde. Es wurde zurückgelassen: Alles, worauf es lief, gehörte Firmen,
die weitergezogen sind, und ein Spiel auf einer geschlossenen Engine stirbt mit der Engine.

## Die Engine wurde gekauft und abgeschaltet

Ballance wurde mit **Virtools** gemacht, einem kommerziellen Werkzeug der Pariser Firma Virtools SA, gegründet
1993. In Virtools wurde die Spiellogik visuell aus fertigen Building Blocks zusammengesetzt, die zu
Verhaltensgraphen verbunden wurden; Programmierer schrieben nur die Blöcke selbst. Das ganze Spiel Ballance
steckt in diesen Graphen.

Im Juli 2005 kaufte Dassault Systèmes Virtools für rund 12 Millionen Euro. Ab 2006 wurde das Produkt als
*3DVIA Virtools* unter der Marke 3DVIA verkauft; danach wurde es nicht mehr weiterentwickelt und um 2009 vom
Markt genommen. Das Dateiformat wurde nie veröffentlicht. Die Laufzeitumgebung gab es nur für Windows.

## Auch das Studio war weg

Cyparade, das kleine deutsche Studio hinter Ballance, hörte um 2006 auf, die Website des Spiels zu pflegen,
2009 ging sie vom Netz. Niemand war mehr da, um das Spiel zu portieren, und keine Engine, auf die man es hätte
portieren können.

## Das Spiel hat beide überlebt

Das Spiel selbst ist nie ganz gestorben. Eine Community hielt es am Leben: Hunderte selbst gebaute Level seit
2009, ein Mod-Loader, Blender-Werkzeuge zum Kartenbau, Speedruns. Im Januar 2024 erschien Ballance neu auf
Steam und GOG (Ziggurat Interactive) — derselbe Windows-Build von 2004, begrenzt auf 1600×1200, mit der
originalen Steuerung.

2026 tat [OpenBallance](https://github.com/emdzej/openballance) von emdzej, was die Eigentümer der Engine nie
getan hätten: Es setzte die Virtools-Laufzeitumgebung, die vom Spiel genutzten Building Blocks und seine
Ipion-Physik neu um und ließ die originalen Spieldaten darauf laufen. Drahtseilakt forkt es, um daraus ein
vollwertiges natives Spiel zu machen.

## Ein bekanntes Muster

Derselbe Eigentümer hat dasselbe mit einem Werkzeug getan, auf das sich Menschen verlassen haben. DraftSight,
ein kostenloses 2D-CAD von Dassault, bekam 2011 eine Linux-Beta; die Linux-Version blieb jahrelang eine Beta,
später wurde das Programm kostenpflichtig, und der Linux-Build war verschwunden. Wer seine Arbeit darauf
aufgebaut hatte, musste Ersatz suchen — heute landen solche Nutzer bei offenen Projekten wie OpenCADStudio.

Die Lehre, auf der der Fork aufbaut: Was Menschen wichtig ist, darf nicht vom guten Willen eines einzelnen
Herstellers abhängen. Daher offener Code, dokumentierte Formate (das offene Levelformat aus dem
[Fahrplan](roadmap.md)) und dauerhafte Unterstützung des Originalformats — damit die Arbeit der Community
weiterläuft, was auch immer mit irgendwelchen Geschäftsplänen geschieht.
