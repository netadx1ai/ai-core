import { SunIcon, MoonIcon, ComputerDesktopIcon } from '@heroicons/react/24/outline';
import { useTheme } from '../hooks/useTheme';
import { Menu, MenuButton, MenuItem, MenuItems } from '@headlessui/react';

export default function ThemeToggle() {
  const { theme, setTheme } = useTheme();

  const themes = [
    { key: 'light', label: 'Light', icon: SunIcon },
    { key: 'dark', label: 'Dark', icon: MoonIcon },
    { key: 'system', label: 'System', icon: ComputerDesktopIcon },
  ] as const;

  const currentTheme = themes.find(t => t.key === theme) || themes[2];
  const CurrentIcon = currentTheme.icon;

  return (
    <Menu as="div" className="relative inline-block text-left">
      <div>
        <MenuButton className="inline-flex justify-center items-center w-8 h-8 text-sm font-medium text-gray-700 dark:text-gray-300 hover:text-gray-900 dark:hover:text-white rounded-md hover:bg-gray-100 dark:hover:bg-dark-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary-500 transition-colors">
          <CurrentIcon className="w-5 h-5" aria-hidden="true" />
        </MenuButton>
      </div>

      <MenuItems className="absolute right-0 z-10 mt-2 w-36 origin-top-right rounded-md bg-white dark:bg-dark-800 shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none">
        <div className="py-1">
          {themes.map((themeOption) => {
            const Icon = themeOption.icon;
            return (
              <MenuItem key={themeOption.key}>
                {({ focus }) => (
                  <button
                    onClick={() => setTheme(themeOption.key)}
                    className={`${
                      focus ? 'bg-gray-100 dark:bg-dark-700 text-gray-900 dark:text-white' : 'text-gray-700 dark:text-gray-300'
                    } ${
                      theme === themeOption.key ? 'bg-primary-50 dark:bg-primary-900/20 text-primary-600 dark:text-primary-400' : ''
                    } group flex items-center px-4 py-2 text-sm w-full text-left transition-colors`}
                  >
                    <Icon className="mr-3 h-4 w-4" aria-hidden="true" />
                    {themeOption.label}
                  </button>
                )}
              </MenuItem>
            );
          })}
        </div>
      </MenuItems>
    </Menu>
  );
}