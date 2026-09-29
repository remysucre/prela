
WITH RECURSIVE UserActivity AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        COALESCE(SUM(p.ViewCount), 0) AS TotalViews,
        COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount,
        COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount,
        COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty
    FROM Users u
    LEFT JOIN Posts p ON u.Id = p.OwnerUserId
    LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) 
    GROUP BY u.Id, u.DisplayName
),
RecentBadges AS (
    SELECT 
        b.UserId,
        b.Name AS BadgeName,
        b.Class,
        ROW_NUMBER() OVER (PARTITION BY b.UserId ORDER BY b.Date DESC, b.Id) AS rn
    FROM Badges b
),
UserBadges AS (
    SELECT 
        rb.UserId,
        STRING_AGG(rb.BadgeName, ', ' ORDER BY rb.rn) AS BadgeList
    FROM RecentBadges rb
    WHERE rb.rn <= 3 
    GROUP BY rb.UserId
)
SELECT 
    ua.UserId,
    ua.DisplayName,
    ua.TotalViews,
    ua.QuestionCount,
    ua.AnswerCount,
    ua.TotalBounty,
    COALESCE(ub.BadgeList, 'No Badges') AS RecentBadges
FROM UserActivity ua
LEFT JOIN UserBadges ub ON ua.UserId = ub.UserId
WHERE ua.TotalViews > 1000 
ORDER BY ua.TotalViews DESC, ua.QuestionCount DESC
FETCH FIRST 10 ROWS ONLY;
