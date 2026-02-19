import type { Metadata } from 'next';
import { Inter } from 'next/font/google';
import { Header } from '@/components/layout/Header';
import { Footer } from '@/components/layout/Footer';
import { ThemeProvider } from '@/components/theme-provider';
import './globals.css';

const inter = Inter({ subsets: ['latin'] });

export const metadata: Metadata = {
  title: 'Legal MCP - Semantische Suche für deutsches und EU-Recht',
  description:
    'Durchsuchen Sie deutsche Gesetze, bayerisches Landesrecht und EU-Verordnungen mit KI-gestützter semantischer Suche.',
  keywords: [
    'Recht',
    'Gesetze',
    'Deutschland',
    'Bayern',
    'EU',
    'Rechtsprechung',
    'Suche',
  ],
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="de" suppressHydrationWarning>
      <body className={inter.className}>
        <ThemeProvider
          attribute="class"
          defaultTheme="system"
          enableSystem
          disableTransitionOnChange
        >
          <div className="relative flex min-h-screen flex-col">
            <Header />
            <main className="flex-1">
              <div className="container py-8">{children}</div>
            </main>
            <Footer />
          </div>
        </ThemeProvider>
      </body>
    </html>
  );
}
