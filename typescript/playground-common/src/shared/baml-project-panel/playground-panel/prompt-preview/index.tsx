'use client'

import { useEffect, useRef, useState } from 'react'
import { PreviewToolbar } from '../preview-toolbar'
import SideBar from '../side-bar'
import { PromptRenderWrapper } from './prompt-render-wrapper'
import TestPanel from './test-panel'
import { ResizableHandle, ResizablePanelGroup } from '@/components/ui/resizable'
import { type ImperativePanelHandle } from 'react-resizable-panels'
import { ResizablePanel } from '@/components/ui/resizable'
import { useAtom, useAtomValue } from 'jotai'
import { areTestsRunningAtom, showEnvDialogAtom } from '../atoms'
import { ThemeProvider } from '../../theme/ThemeProvider'
import { EnvironmentVariablesDialog } from '../side-bar/env-vars'
import { SidebarInset, SidebarProvider } from '@/components/ui/sidebar'
import { isSidebarOpenAtom } from '../side-bar'

const PromptPreview = ({ isEmbed = false }: { isEmbed?: boolean }) => {
  const areTestsRunning = useAtomValue(areTestsRunningAtom)
  const ref = useRef<ImperativePanelHandle>(null)
  const [showEnvDialog, setShowEnvDialog] = useAtom(showEnvDialogAtom)
  const [isOpen, setIsOpen] = useAtom(isSidebarOpenAtom)

  const handleResize = () => {
    if (ref.current) {
      if (areTestsRunning) {
        // expand the test panel to 70% of the height
        console.log('ref.current.getSize()', ref.current.getSize())
        if (ref.current.getSize() < 60) {
          console.log('resizing to 70')
          ref.current.resize(80)
        }
      } else {
        // ref.current.resize(20);
      }
    }
  }

  useEffect(() => {
    handleResize()
  }, [areTestsRunning])

  return (
    <SidebarProvider >
        <SidebarInset>
      <div className='flex w-full h-full bg-background text-foreground'>
        <div
          className='flex overflow-x-auto flex-col w-full h-full gap-2'
        >

          <EnvironmentVariablesDialog showEnvDialog={showEnvDialog} setShowEnvDialog={setShowEnvDialog} />
          <PreviewToolbar />
          <PromptRenderWrapper />
          <TestPanel />
        </div>
      </div>
          </SidebarInset>
          <SideBar />
    </SidebarProvider>
  )
}

export default PromptPreview
