import { Component, OnInit, signal } from '@angular/core';
import { Router, RouterOutlet } from '@angular/router';

@Component({
  selector: 'app-root',
  imports: [RouterOutlet],
  templateUrl: './app.html',
  styleUrl: './app.scss'
})
export class App implements OnInit {
  protected readonly title = signal('mail-agent');
  constructor(private readonly router: Router) {}
  ngOnInit() {
    const params = new URLSearchParams(window.location.search);
    if (params.get('view') === 'alert') {
      this.router.navigateByUrl('/alert');
    }
  }
}
