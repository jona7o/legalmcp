import { Separator } from '@/components/ui/separator';

export function Footer() {
  return (
    <footer className="mt-auto border-t bg-background">
      <div className="container py-8">
        <div className="grid gap-8 md:grid-cols-2 lg:grid-cols-3">
          <div>
            <h3 className="mb-3 text-sm font-semibold">Legal MCP</h3>
            <p className="text-sm text-muted-foreground">
              Semantische Suche für deutsches und EU-Recht
            </p>
          </div>
          
          <div>
            <h3 className="mb-3 text-sm font-semibold">Rechtliche Hinweise</h3>
            <p className="text-xs text-muted-foreground">
              Die Informationen auf dieser Plattform dienen ausschließlich zu 
              Informationszwecken und stellen keine Rechtsberatung dar.
            </p>
          </div>
          
          <div>
            <h3 className="mb-3 text-sm font-semibold">Haftungsausschluss</h3>
            <p className="text-xs text-muted-foreground">
              Trotz sorgfältiger inhaltlicher Kontrolle übernehmen wir keine 
              Haftung für die Richtigkeit, Vollständigkeit und Aktualität der 
              bereitgestellten Informationen.
            </p>
          </div>
        </div>
        
        <Separator className="my-6" />
        
        <div className="flex flex-col gap-2 text-center text-xs text-muted-foreground md:flex-row md:justify-between">
          <p>© 2026 Legal MCP. Alle Rechte vorbehalten.</p>
          <p>
            Keine Rechtsberatung • Keine Gewähr für Aktualität und Vollständigkeit
          </p>
        </div>
      </div>
    </footer>
  );
}
