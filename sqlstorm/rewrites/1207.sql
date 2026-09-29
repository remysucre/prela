
WITH RankedPosts AS (
    SELECT
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.Score,
        p.ViewCount,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.Id) AS Rank
    FROM
        Posts p
    WHERE
        p.PostTypeId = 1
        AND p.Score > 0
),
PostStatistics AS (
    SELECT
        u.Id AS UserId,
        u.DisplayName,
        COUNT(DISTINCT p.Id) AS TotalQuestions,
        COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 END), 0) AS GoldBadges,
        COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 END), 0) AS SilverBadges,
        COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 END), 0) AS BronzeBadges,
        COALESCE(AVG(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END), 0) AS AverageCloseReasons,
        COALESCE(AVG(p.ViewCount), 0) AS AverageViews
    FROM
        Users u
    LEFT JOIN
        Posts p ON p.OwnerUserId = u.Id
    LEFT JOIN
        Badges b ON b.UserId = u.Id
    LEFT JOIN
        PostHistory ph ON ph.UserId = u.Id AND ph.PostId = p.Id
    WHERE
        p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
    GROUP BY
        u.Id, u.DisplayName
)
SELECT
    ps.DisplayName,
    ps.TotalQuestions,
    ps.GoldBadges,
    ps.SilverBadges,
    ps.BronzeBadges,
    ps.AverageCloseReasons,
    ps.AverageViews,
    rp.Title,
    rp.CreationDate,
    rp.Score,
    rp.ViewCount
FROM
    PostStatistics ps
LEFT JOIN
    RankedPosts rp ON ps.UserId = rp.PostId
WHERE
    ps.TotalQuestions > 10
    AND rp.Rank <= 5
ORDER BY
    ps.TotalQuestions DESC, ps.DisplayName
LIMIT 50 OFFSET 0;
