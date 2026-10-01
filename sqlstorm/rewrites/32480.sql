WITH RecursivePostHistory AS (
 SELECT
 ph.Id,
 ph.PostId,
 ph.CreationDate,
 ph.UserId,
 ph.Comment,
 ph.PostHistoryTypeId,
 ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC, ph.Id DESC) as rn
 FROM PostHistory ph
),
UserBadgeCounts AS (
 SELECT
 b.UserId,
 COUNT(CASE WHEN b.Class = 1 THEN 1 END) as GoldBadges,
 COUNT(CASE WHEN b.Class = 2 THEN 1 END) as SilverBadges,
 COUNT(CASE WHEN b.Class = 3 THEN 1 END) as BronzeBadges
 FROM Badges b
 GROUP BY b.UserId
),
UserVoteSummary AS (
 SELECT
 v.UserId,
 SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
 SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
 FROM Votes v
 GROUP BY v.UserId
),
PostMetrics AS (
 SELECT
 p.Id AS PostId,
 p.OwnerUserId,
 COUNT(DISTINCT c.Id) AS CommentCount,
 COUNT(DISTINCT v.Id) AS VoteCount,
 SUM(p.ViewCount) AS TotalViews,
 SUM(p.Score) AS TotalScore
 FROM Posts p
 LEFT JOIN Comments c ON c.PostId = p.Id
 LEFT JOIN Votes v ON v.PostId = p.Id
 WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
 GROUP BY p.Id, p.OwnerUserId
)
SELECT
 u.Id AS UserId,
 u.DisplayName,
 ub.GoldBadges,
 ub.SilverBadges,
 ub.BronzeBadges,
 ups.UpVotes,
 ups.DownVotes,
 pm.PostId,
 pm.CommentCount,
 pm.VoteCount,
 pm.TotalViews,
 pm.TotalScore,
 (SELECT ARRAY_AGG(ph.Comment ORDER BY ph.CreationDate DESC, ph.Id DESC)
 FROM RecursivePostHistory ph
 WHERE ph.PostId = pm.PostId) AS RecentHistoryComments
FROM Users u
LEFT JOIN UserBadgeCounts ub ON u.Id = ub.UserId
LEFT JOIN UserVoteSummary ups ON u.Id = ups.UserId
LEFT JOIN PostMetrics pm ON u.Id = pm.OwnerUserId
WHERE (ub.GoldBadges > 0 OR ub.SilverBadges > 0 OR ub.BronzeBadges > 0)
 AND pm.VoteCount > 10
ORDER BY pm.TotalScore DESC, u.Reputation DESC;
