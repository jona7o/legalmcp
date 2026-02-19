import { notFound } from 'next/navigation';
import { ArrowLeft } from 'lucide-react';
import Link from 'next/link';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import { Calendar, Gavel, ExternalLink } from 'lucide-react';
import { apiClient } from '@/lib/api';
import { formatDate } from '@/lib/utils';

interface CasePageProps {
  params: {
    id: string;
  };
}

async function getCase(id: string) {
  try {
    return await apiClient.getCase(id);
  } catch (error) {
    return null;
  }
}

export default async function CasePage({ params }: CasePageProps) {
  const caseDoc = await getCase(params.id);

  if (!caseDoc) {
    notFound();
  }

  return (
    <div className="space-y-6">
      <div>
        <Link href="/">
          <Button variant="ghost" size="sm" className="gap-2">
            <ArrowLeft className="h-4 w-4" />
            Zurück zur Suche
          </Button>
        </Link>
      </div>

      {/* Header */}
      <Card>
        <CardHeader>
          <div className="flex items-start justify-between gap-4">
            <div className="flex items-start gap-3">
              <Gavel className="mt-1 h-6 w-6 text-primary" />
              <div>
                <CardTitle className="text-3xl">{caseDoc.title}</CardTitle>
                <p className="mt-2 text-lg text-muted-foreground">
                  {caseDoc.court}
                </p>
              </div>
            </div>
            <Badge variant="outline" className="text-lg px-4 py-2">
              {caseDoc.jurisdiction}
            </Badge>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex flex-wrap gap-4 text-sm text-muted-foreground">
            <div className="flex items-center gap-2">
              <Calendar className="h-4 w-4" />
              <span>{formatDate(caseDoc.date)}</span>
            </div>
            {caseDoc.fileNumber && (
              <div className="flex items-center gap-2">
                <span>Az.: {caseDoc.fileNumber}</span>
              </div>
            )}
          </div>

          {caseDoc.metadata.ecli && (
            <div>
              <strong className="text-sm">ECLI:</strong>
              <p className="mt-1 text-sm text-muted-foreground font-mono">
                {caseDoc.metadata.ecli}
              </p>
            </div>
          )}

          {caseDoc.metadata.citation && (
            <div>
              <strong className="text-sm">Fundstelle:</strong>
              <p className="mt-1 text-sm text-muted-foreground">
                {caseDoc.metadata.citation}
              </p>
            </div>
          )}

          {caseDoc.metadata.url && (
            <a
              href={caseDoc.metadata.url}
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-2 text-sm text-primary hover:underline"
            >
              <ExternalLink className="h-4 w-4" />
              Originalquelle ansehen
            </a>
          )}
        </CardContent>
      </Card>

      {/* Summary */}
      {caseDoc.summary && (
        <Card>
          <CardHeader>
            <CardTitle>Zusammenfassung</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm">{caseDoc.summary}</p>
          </CardContent>
        </Card>
      )}

      {/* Headnotes */}
      {caseDoc.headnotes && caseDoc.headnotes.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Leitsätze</CardTitle>
          </CardHeader>
          <CardContent>
            <ol className="list-decimal list-inside space-y-2">
              {caseDoc.headnotes.map((headnote, index) => (
                <li key={index} className="text-sm">
                  {headnote}
                </li>
              ))}
            </ol>
          </CardContent>
        </Card>
      )}

      {/* Full Text */}
      <Card>
        <CardHeader>
          <CardTitle>Volltext</CardTitle>
        </CardHeader>
        <CardContent>
          <div
            className="prose prose-sm max-w-none dark:prose-invert"
            dangerouslySetInnerHTML={{ __html: caseDoc.fullText }}
          />
        </CardContent>
      </Card>

      {/* References */}
      {caseDoc.references && caseDoc.references.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Zitierte Normen</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              {caseDoc.references.map((ref, index) => (
                <div key={index} className="flex items-start gap-2 text-sm">
                  <Badge variant="outline" className="mt-0.5">
                    {ref.type === 'law' ? 'Gesetz' : 'Urteil'}
                  </Badge>
                  <div>
                    <Link
                      href={
                        ref.type === 'law'
                          ? `/law/${ref.id}`
                          : `/case/${ref.id}`
                      }
                      className="font-semibold text-primary hover:underline"
                    >
                      {ref.citation}
                    </Link>
                    {ref.context && (
                      <p className="mt-1 text-muted-foreground">
                        {ref.context}
                      </p>
                    )}
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}

      {/* Disclaimer */}
      <Card className="border-yellow-500/50 bg-yellow-50 dark:bg-yellow-950/20">
        <CardContent className="pt-6">
          <p className="text-sm text-muted-foreground">
            <strong>Hinweis:</strong> Die dargestellten Urteile dienen
            ausschließlich der Information und stellen keine Rechtsberatung
            dar. Für rechtsverbindliche Auskünfte wenden Sie sich bitte an
            einen Rechtsanwalt.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
