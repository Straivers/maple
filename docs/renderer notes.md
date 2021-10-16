
multi-window rendering
    each window may be on different monitors
        different Vsync timings

    event thread per window, allows thread to block (on input, frame acquire)

    render thread's only job is to arbitrate access to graphics queue

        render_conn.send(SubmitAndPresent{...});
        let render_ack = render_conn.recv();

    communication handled by 2 connections
        renderer -> window
        window -> renderer

        renderer does not store connection to window
            to_window must be passed to renderer on every send() op.

    window thread needs access to VkInstance
        create swapchain (get fences & semaphores)
        wait for swapchain image
        record command buffer

    windows are drawn every...
        no more than refresh rate
        whenever there are input events that update UI state
        on animation ticks (at refresh rate)
            need to be able to get monitor refresh rate & update when window moves between monitors.

    renderer messages:
        SubmitAndPresent
