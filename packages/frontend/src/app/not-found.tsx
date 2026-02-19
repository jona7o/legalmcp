import Link from 'next/link';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { FileQuestion } from 'lucide-react';

export default function NotFound() {
  return (
    <div className="flex items-center justify-center min-h-[60vh]">
      <Card className="max-w-md w-full">
        <CardHeader className="text-center">
          <div className="flex justify-center mb-4">
            <FileQuestion className="h-16 w-16 text-muted-foreground" />
          </div>
          <CardTitle className="text-2xl">Seite nicht gefunden</CardTitle>
        </CardHeader>
        <CardContent className="text-center space-y-4">
          <p className="text-muted-foreground">
            Die von Ihnen gesuchte Seite existiert nicht oder wurde verschoben.
          </p>
          <Link href="/">
            <Button>Zurück zur Startseite</Button>
          </Link>
        </CardContent>
      </Card>
    </div>
  );
}
