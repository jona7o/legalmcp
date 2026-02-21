'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { Scale, Moon, Sun } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useTheme } from 'next-themes';
import { useTranslations } from 'next-intl';
import { useEffect, useState } from 'react';

export function Header() {
  const pathname = usePathname();
  const { theme, setTheme } = useTheme();
  const [mounted, setMounted] = useState(false);
  const t = useTranslations('header');

  useEffect(() => {
    setMounted(true);
  }, []);

  // Strip locale prefix for active path matching (e.g. /de/changes → /changes)
  const pathWithoutLocale = pathname.replace(/^\/[a-z]{2}(\/|$)/, '/');

  return (
    <header className="sticky top-0 z-50 w-full border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
      <div className="container flex h-16 items-center">
        <div className="mr-4 flex">
          <Link href="/" className="mr-6 flex items-center space-x-2">
            <Scale className="h-6 w-6" />
            <span className="hidden font-bold sm:inline-block">Legal MCP</span>
          </Link>
          <nav className="flex items-center space-x-6 text-sm font-medium">
            <Link
              href="/"
              className={`transition-colors hover:text-foreground/80 ${
                pathWithoutLocale === '/' ? 'text-foreground' : 'text-foreground/60'
              }`}
            >
              {t('search')}
            </Link>
            <Link
              href="/chat"
              className={`transition-colors hover:text-foreground/80 ${
                pathWithoutLocale === '/chat' ? 'text-foreground' : 'text-foreground/60'
              }`}
            >
              {t('aiAssistant')}
            </Link>
            <Link
              href="/changes"
              className={`transition-colors hover:text-foreground/80 ${
                pathWithoutLocale === '/changes' ? 'text-foreground' : 'text-foreground/60'
              }`}
            >
              {t('changes')}
            </Link>
          </nav>
        </div>
        <div className="ml-auto flex items-center space-x-4">
          {mounted && (
            <Button
              variant="ghost"
              size="icon"
              onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}
            >
              {theme === 'dark' ? (
                <Sun className="h-5 w-5" />
              ) : (
                <Moon className="h-5 w-5" />
              )}
              <span className="sr-only">{t('toggleTheme')}</span>
            </Button>
          )}
        </div>
      </div>
    </header>
  );
}
